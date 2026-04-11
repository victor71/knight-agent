//! IPC-based Daemon Client
//!
//! Implements DaemonClient trait using IPC transport to communicate
//! with the daemon process.

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use crate::event::{AppEvent, SystemHealth, SystemStatusSnapshot};
use crate::state::SessionListItem;

use super::{DaemonClient, DaemonClientError, DaemonClientResult};

/// IPC-based daemon client
pub struct IpcDaemonClient {
    /// IPC client for communication (wrapped in Mutex for mutable access)
    ipc_client: Arc<Mutex<ipc_contract::IpcClient>>,
    /// Daemon address
    daemon_addr: String,
}

impl IpcDaemonClient {
    /// Create a new IPC daemon client and connect to daemon
    pub async fn new(daemon_addr: String) -> Result<Self, DaemonClientError> {
        // Parse address
        let socket_addr: std::net::SocketAddr = daemon_addr
            .parse()
            .map_err(|e| DaemonClientError::ConnectionFailed(format!("Invalid address: {}", e)))?;

        // Create client config
        let config = ipc_contract::IpcClientConfig {
            server_addr: socket_addr,
            connect_timeout_ms: 5000,
            request_timeout_ms: 30000,
            event_channel_size: 100,
        };

        // Create client
        let mut ipc_client = ipc_contract::IpcClient::new(config);

        // Connect to daemon
        ipc_client
            .connect()
            .await
            .map_err(|e| DaemonClientError::ConnectionFailed(e.to_string()))?;

        info!("Connected to daemon at {}", daemon_addr);

        Ok(Self {
            ipc_client: Arc::new(Mutex::new(ipc_client)),
            daemon_addr,
        })
    }

    /// Try to connect to an existing daemon, returning error if failed
    pub async fn try_connect(daemon_addr: &str) -> Result<Self, DaemonClientError> {
        Self::new(daemon_addr.to_string()).await
    }
}

#[async_trait]
impl DaemonClient for IpcDaemonClient {
    fn handle_input(
        &self,
        input: String,
        session_id: String,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<router::HandleInputResult>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            let params = serde_json::json!({
                "input": input,
                "session_id": session_id,
            });

            let client = client.lock().await;
            let response = client
                .request("handle_input".to_string(), params)
                .await
                .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

            // Parse response
            let response_obj: serde_json::Value = response;
            let to_agent = response_obj
                .get("to_agent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let resp_value = response_obj.get("response").ok_or_else(|| {
                DaemonClientError::InternalError("Missing response field".to_string())
            })?;

            let success = resp_value
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let message = resp_value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let data = resp_value.get("data").cloned();
            let error = resp_value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Build HandleInputResult
            let router_response = router::RouterResponse {
                success,
                message,
                data,
                error,
                to_agent,
            };

            Ok(router::HandleInputResult {
                response: router_response,
                to_agent,
                should_exit: false,
            })
        })
    }

    fn send_message(
        &self,
        session_id: &str,
        content: String,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>> {
        self.send_message_streaming(session_id, content, None)
    }

    fn send_message_streaming(
        &self,
        session_id: &str,
        content: String,
        stream_callback: Option<Box<dyn Fn(String) -> bool + Send + Sync>>,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>> {
        let client = self.ipc_client.clone();
        let session_id = session_id.to_string();

        Box::pin(async move {
            let params = serde_json::json!({
                "session_id": session_id,
                "content": content,
            });

            let mut client = client.lock().await;

            // Use streaming if callback provided
            if let Some(callback) = stream_callback {
                match client
                    .request_streaming("send_message".to_string(), params)
                    .await
                {
                    Ok((mut chunk_rx, response_rx)) => {
                        // Spawn task to handle chunks with timeout
                        let chunk_tx = Arc::new(AtomicBool::new(true));
                        let chunk_tx_for_task = chunk_tx.clone();

                        let handle = tokio::spawn(async move {
                            while chunk_tx_for_task.load(Ordering::SeqCst) {
                                tokio::select! {
                                    chunk = chunk_rx.recv() => {
                                        match chunk {
                                            Some(c) => {
                                                if !callback(c.chunk) {
                                                    break;
                                                }
                                            }
                                            None => break, // Channel closed
                                        }
                                    }
                                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                                        // Timeout - stop waiting for more chunks
                                        break;
                                    }
                                }
                            }
                        });

                        // Wait for final response with timeout
                        let response = tokio::time::timeout(
                            std::time::Duration::from_secs(120),
                            client.wait_for_stream_response(response_rx),
                        )
                        .await
                        .map_err(|_| {
                            DaemonClientError::InternalError(
                                "Streaming timed out after 120 seconds".to_string(),
                            )
                        })?
                        .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

                        // Signal chunk handler to stop and wait for it
                        chunk_tx.store(false, Ordering::SeqCst);
                        let _ = handle.await;

                        let response_obj: serde_json::Value = response;
                        let result = response_obj
                            .get("response")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        Ok(result)
                    }
                    Err(e) => {
                        // Fall back to regular request if streaming fails
                        let params = serde_json::json!({
                            "session_id": session_id,
                            "content": content,
                        });
                        let response = client
                            .request("send_message".to_string(), params)
                            .await
                            .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

                        let response_obj: serde_json::Value = response;
                        let result = response_obj
                            .get("response")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        Ok(result)
                    }
                }
            } else {
                // No callback, use regular request
                let response = client
                    .request("send_message".to_string(), params)
                    .await
                    .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

                let response_obj: serde_json::Value = response;
                let result = response_obj
                    .get("response")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Ok(result)
            }
        })
    }

    fn list_sessions(
        &self,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<Vec<SessionListItem>>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            let params = serde_json::json!({});

            let client = client.lock().await;
            let response = client
                .request("list_sessions".to_string(), params)
                .await
                .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

            let response_obj: serde_json::Value = response;
            let sessions_array = response_obj
                .get("sessions")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    DaemonClientError::InternalError("Missing sessions array".to_string())
                })?;

            let mut sessions = Vec::new();
            for session_value in sessions_array {
                let id = session_value
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let name = session_value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let status = session_value
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let created_at_str = session_value
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let message_count = session_value
                    .get("message_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                // Parse created_at
                let created_at = created_at_str
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Local))
                    .unwrap_or_else(chrono::Local::now);

                sessions.push(SessionListItem {
                    id,
                    name,
                    status,
                    created_at,
                    message_count,
                });
            }

            Ok(sessions)
        })
    }

    fn create_session(
        &self,
        name: Option<String>,
        workspace: String,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            let mut params = serde_json::json!({
                "workspace": workspace,
            });

            if let Some(name) = name {
                params["name"] = serde_json::json!(name);
            }

            let client = client.lock().await;
            let response = client
                .request("create_session".to_string(), params)
                .await
                .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

            let response_obj: serde_json::Value = response;
            let session_id = response_obj
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            Ok(session_id)
        })
    }

    fn switch_session(
        &self,
        session_id: &str,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<()>> + Send>> {
        let client = self.ipc_client.clone();
        let session_id = session_id.to_string();

        Box::pin(async move {
            let params = serde_json::json!({
                "session_id": session_id,
            });

            let client = client.lock().await;
            let _response = client
                .request("switch_session".to_string(), params)
                .await
                .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

            Ok(())
        })
    }

    fn get_system_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<SystemStatusSnapshot>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            let params = serde_json::json!({});

            let client = client.lock().await;
            let response = client
                .request("get_system_status".to_string(), params)
                .await
                .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;

            let response_obj: serde_json::Value = response;

            let stage = response_obj
                .get("stage")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u8;

            let initialized = response_obj
                .get("initialized")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let ready = response_obj
                .get("ready")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let module_count = response_obj
                .get("module_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let initialized_count = response_obj
                .get("initialized_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let stage_name = bootstrap::BootstrapStage::from_u8(stage)
                .map(|s| s.name().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            Ok(SystemStatusSnapshot {
                system_status: if ready {
                    SystemHealth::Healthy
                } else {
                    SystemHealth::Degraded
                },
                stage: stage_name,
                module_count,
                initialized_count,
                uptime: std::time::Duration::ZERO,
                cpu_usage: 0.0,
                memory_usage: 0,
            })
        })
    }

    fn subscribe_events(
        &self,
    ) -> Pin<Box<dyn Future<Output = DaemonClientResult<mpsc::UnboundedReceiver<AppEvent>>> + Send>>
    {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement event subscription via IPC
            // For now, return empty channel
            let (_tx, rx) = mpsc::unbounded_channel();
            Ok(rx)
        })
    }

    fn shutdown(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<()>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            let params = serde_json::json!({});
            let client = client.lock().await;
            let _ = client
                .request("shutdown".to_string(), params)
                .await
                .map_err(|e| DaemonClientError::InternalError(e.to_string()))?;
            Ok(())
        })
    }
}
