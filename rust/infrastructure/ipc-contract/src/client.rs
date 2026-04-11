//! IPC Client
//!
//! Connects to IPC server and sends requests.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::error::{IPCError, IPCResult};
use crate::transport::{Connection, TcpTransport, Transport};
use crate::types::{
    BaseMessage, MessageType, NotificationMessage, RequestMessage, ResponseMessage,
    StreamChunkMessage,
};

/// IPC client configuration
#[derive(Debug, Clone)]
pub struct IpcClientConfig {
    /// Server address
    pub server_addr: std::net::SocketAddr,
    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u64,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Event channel size
    pub event_channel_size: usize,
}

impl Default for IpcClientConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:7357".parse().unwrap(),
            connect_timeout_ms: 5000,
            request_timeout_ms: 30000,
            event_channel_size: 100,
        }
    }
}

/// Pending request
struct PendingRequest {
    response_tx: oneshot::Sender<ResponseMessage>,
    chunk_tx: Option<mpsc::Sender<StreamChunkMessage>>,
}

/// IPC client
pub struct IpcClient {
    config: IpcClientConfig,
    transport: TcpTransport,
    connection: Arc<RwLock<Option<Box<dyn Connection>>>>,
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
    event_tx: mpsc::UnboundedSender<ClientEvent>,
    response_task_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Client events
#[derive(Debug, Clone)]
pub enum ClientEvent {
    /// Connected to server
    Connected,
    /// Disconnected from server
    Disconnected,
    /// Request sent
    RequestSent { request_id: String, method: String },
    /// Response received
    ResponseReceived { request_id: String },
    /// Stream chunk received
    StreamChunk { request_id: String, chunk: String },
    /// Notification received
    Notification {
        event: String,
        data: serde_json::Value,
    },
    /// Error occurred
    Error { error: String },
}

impl IpcClient {
    /// Create new IPC client
    pub fn new(config: IpcClientConfig) -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();

        Self {
            config,
            transport: TcpTransport::new(),
            connection: Arc::new(RwLock::new(None)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            response_task_handle: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Connect to server
    pub async fn connect(&mut self) -> IPCResult<()> {
        let conn = tokio::time::timeout(
            tokio::time::Duration::from_millis(self.config.connect_timeout_ms),
            self.transport.connect(&self.config.server_addr),
        )
        .await
        .map_err(|_| IPCError::Timeout(self.config.connect_timeout_ms))?
        .map_err(|e| IPCError::ConnectionFailed(e.to_string()))?;

        {
            let mut connection = self.connection.write().await;
            *connection = Some(Box::new(conn));
        }

        let _ = self.event_tx.send(ClientEvent::Connected);

        // Spawn response handler
        self.spawn_response_handler().await;

        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        let connection = self.connection.read().await;
        connection.is_some()
    }

    /// Send a request and wait for response
    pub async fn request(
        &self,
        method: String,
        params: serde_json::Value,
    ) -> IPCResult<serde_json::Value> {
        let response_rx = self.request_internal(method, params, None).await?;
        self.wait_for_stream_response(response_rx).await
    }

    /// Send a streaming request and get chunk receiver
    pub async fn request_streaming(
        &self,
        method: String,
        params: serde_json::Value,
    ) -> IPCResult<(
        mpsc::Receiver<StreamChunkMessage>,
        oneshot::Receiver<ResponseMessage>,
    )> {
        let (chunk_tx, chunk_rx) = mpsc::channel(256);
        let response_rx = self
            .request_internal(method, params, Some(chunk_tx))
            .await?;
        Ok((chunk_rx, response_rx))
    }

    /// Internal request implementation
    async fn request_internal(
        &self,
        method: String,
        params: serde_json::Value,
        chunk_tx: Option<mpsc::Sender<StreamChunkMessage>>,
    ) -> IPCResult<oneshot::Receiver<ResponseMessage>> {
        // Create request message first to get its ID
        let mut request = RequestMessage {
            base: BaseMessage::new(MessageType::Request),
            method: method.clone(),
            params,
            options: None,
        };

        // Mark as streaming request if chunk_tx provided
        if chunk_tx.is_some() {
            request.options = Some(crate::types::RequestOptions {
                stream: Some(true),
                ..Default::default()
            });
        }

        // Use the message ID for tracking
        let request_id = request.base.id.clone();

        let (response_tx, response_rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(
                request_id.clone(),
                PendingRequest {
                    response_tx,
                    chunk_tx,
                },
            );
        }

        let request_str = serde_json::to_string(&request)
            .map_err(|e| IPCError::ParseError(format!("JSON error: {}", e)))?;
        // Send request directly
        {
            let mut conn = self.connection.write().await;
            if let Some(conn) = conn.as_mut() {
                conn.send(request_str)
                    .await
                    .map_err(|e| IPCError::SendFailed(e.to_string()))?;
            } else {
                drop(conn);
                self.pending_requests.write().await.remove(&request_id);
                return Err(IPCError::ConnectionFailed("Not connected".to_string()));
            }
        }

        // Yield to let response handler acquire lock
        tokio::task::yield_now().await;

        let _ = self.event_tx.send(ClientEvent::RequestSent {
            request_id: request_id.clone(),
            method,
        });

        Ok(response_rx)
    }

    /// Wait for streaming response completion
    pub async fn wait_for_stream_response(
        &self,
        response_rx: oneshot::Receiver<ResponseMessage>,
    ) -> IPCResult<serde_json::Value> {
        let response = tokio::time::timeout(
            tokio::time::Duration::from_millis(self.config.request_timeout_ms),
            response_rx,
        )
        .await
        .map_err(|_| IPCError::Timeout(self.config.request_timeout_ms))?
        .map_err(|_| IPCError::ReceiveFailed("Response channel closed".to_string()))?;

        if let Some(error) = response.error {
            Err(IPCError::InternalError(format!(
                "{}: {}",
                error.message,
                error.details.unwrap_or(serde_json::Value::Null)
            )))
        } else if let Some(result) = response.result {
            Ok(result)
        } else {
            Err(IPCError::ParseError("Empty response".to_string()))
        }
    }

    /// Send a notification (no response expected)
    pub async fn notify(&self, event: String, data: serde_json::Value) -> IPCResult<()> {
        let notification = NotificationMessage::new(event, data);

        let notification_str = serde_json::to_string(&notification)
            .map_err(|e| IPCError::ParseError(format!("JSON error: {}", e)))?;

        let mut conn = self.connection.write().await;
        if let Some(conn) = conn.as_mut() {
            conn.send(notification_str)
                .await
                .map_err(|e| IPCError::SendFailed(e.to_string()))?;
        } else {
            return Err(IPCError::ConnectionFailed("Not connected".to_string()));
        }

        Ok(())
    }

    /// Subscribe to client events
    pub fn subscribe_events(&self) -> mpsc::UnboundedReceiver<ClientEvent> {
        let (_tx, rx) = mpsc::unbounded_channel();
        // In real implementation, forward events to new subscriber
        rx
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) -> IPCResult<()> {
        // Stop response handler task
        let mut handle_guard = self.response_task_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }

        let mut connection = self.connection.write().await;
        if let Some(mut conn) = connection.take() {
            conn.close().await?;
        }

        let _ = self.event_tx.send(ClientEvent::Disconnected);

        Ok(())
    }

    /// Spawn response handler task
    async fn spawn_response_handler(&self) {
        let connection = self.connection.clone();
        let pending_requests = self.pending_requests.clone();
        let event_tx = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            loop {
                // Try to acquire write lock, but don't wait forever
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

                if let Ok(mut conn_guard) =
                    tokio::time::timeout(tokio::time::Duration::from_millis(5), connection.write())
                        .await
                {
                    if conn_guard.is_some() {
                        // Try to receive with short timeout
                        let recv_result = tokio::time::timeout(
                            tokio::time::Duration::from_millis(5),
                            conn_guard.as_mut().unwrap().recv(),
                        )
                        .await;

                        match recv_result {
                            Ok(Ok(Some(msg_str))) => {
                                drop(conn_guard); // Release lock before processing

                                // Determine message type from the "type" field before deserializing
                                // This is critical because ResponseMessage and StreamChunkMessage
                                // can be deserialized from each other (shared fields, optional fields)
                                let msg_type: Option<MessageType> =
                                    serde_json::from_str::<serde_json::Value>(&msg_str)
                                        .ok()
                                        .and_then(|v| v.get("type").cloned())
                                        .and_then(|v| serde_json::from_value(v).ok());

                                match msg_type {
                                    Some(MessageType::StreamChunk) => {
                                        // Parse as StreamChunkMessage
                                        if let Ok(chunk) =
                                            serde_json::from_str::<StreamChunkMessage>(&msg_str)
                                        {
                                            let request_id = chunk.request_id.clone();
                                            tracing::debug!("IPC client received chunk for request {}: {} chars", request_id, chunk.chunk.len());

                                            // Forward chunk to pending stream request
                                            let chunk_tx_opt = {
                                                let pending_requests =
                                                    pending_requests.read().await;
                                                pending_requests
                                                    .get(&request_id)
                                                    .and_then(|p| p.chunk_tx.as_ref().cloned())
                                            };

                                            if let Some(chunk_tx) = chunk_tx_opt {
                                                let _ = chunk_tx.send(chunk.clone());
                                                let _ = event_tx.send(ClientEvent::StreamChunk {
                                                    request_id,
                                                    chunk: chunk.chunk,
                                                });
                                                tracing::debug!(
                                                    "IPC client forwarded chunk to chunk_rx"
                                                );
                                            } else {
                                                tracing::warn!(
                                                    "Received chunk for unknown request: {}",
                                                    chunk.request_id
                                                );
                                            }
                                        }
                                    }
                                    Some(MessageType::Response) => {
                                        // Parse as ResponseMessage
                                        if let Ok(response) =
                                            serde_json::from_str::<ResponseMessage>(&msg_str)
                                        {
                                            let request_id = response.request_id.clone();

                                            // Remove from pending and send response
                                            let pending =
                                                pending_requests.write().await.remove(&request_id);

                                            if let Some(pending) = pending {
                                                let _ = pending.response_tx.send(response);
                                            } else {
                                                tracing::warn!(
                                                    "Received response for unknown request: {}",
                                                    request_id
                                                );
                                            }
                                        }
                                    }
                                    Some(MessageType::Notification) => {
                                        if let Ok(_notification) =
                                            serde_json::from_str::<NotificationMessage>(&msg_str)
                                        {
                                            let _ = event_tx.send(ClientEvent::Error {
                                                error: "Notification handling not implemented"
                                                    .to_string(),
                                            });
                                        }
                                    }
                                    _ => {
                                        tracing::warn!(
                                            "Received unknown message type: {}",
                                            &msg_str[..msg_str.len().min(100)]
                                        );
                                    }
                                }
                            }
                            Ok(Ok(None)) => {
                                // Connection closed
                                let _ = event_tx.send(ClientEvent::Disconnected);
                                break;
                            }
                            Ok(Err(e)) => {
                                let _ = event_tx.send(ClientEvent::Error {
                                    error: e.to_string(),
                                });
                                break;
                            }
                            Err(_) => {
                                // Timeout on recv, loop again
                            }
                        }
                    } else {
                        // Connection closed
                        break;
                    }
                } else {
                    // Couldn't get lock, try again
                }
            }
        });

        let mut handle_guard = self.response_task_handle.lock().await;
        *handle_guard = Some(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::IpcServer;

    #[tokio::test]
    async fn test_client_connect() {
        // Start server
        let config = crate::server::IpcServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        let mut server = IpcServer::new(config);
        let bound_addr = server.start().await.unwrap();

        // Connect client
        let client_config = IpcClientConfig {
            server_addr: bound_addr,
            ..Default::default()
        };

        let mut client = IpcClient::new(client_config);
        client.connect().await.unwrap();
        assert!(client.is_connected().await);

        client.disconnect().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_client_request() {
        // Start server with echo handler
        let config = crate::server::IpcServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        let mut server = IpcServer::new(config);
        server
            .register("echo", |params| async move { Ok(params) })
            .await
            .unwrap();

        let bound_addr = server.start().await.unwrap();

        // Connect client
        let client_config = IpcClientConfig {
            server_addr: bound_addr,
            ..Default::default()
        };

        let mut client = IpcClient::new(client_config);
        client.connect().await.unwrap();

        // Send request
        let result = client
            .request("echo".to_string(), serde_json::json!("test"))
            .await
            .unwrap();

        assert_eq!(result, "test");

        client.disconnect().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_client_request_not_found() {
        // Start server without handlers
        let config = crate::server::IpcServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        let mut server = IpcServer::new(config);
        let bound_addr = server.start().await.unwrap();

        // Connect client
        let client_config = IpcClientConfig {
            server_addr: bound_addr,
            ..Default::default()
        };

        let mut client = IpcClient::new(client_config);
        client.connect().await.unwrap();

        // Send request to non-existent method
        let result = client
            .request("unknown".to_string(), serde_json::json!(null))
            .await;

        assert!(matches!(result, Err(IPCError::InternalError(_))));

        client.disconnect().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_client_connection_failed() {
        let client_config = IpcClientConfig {
            server_addr: "127.0.0.1:54321".parse().unwrap(),
            ..Default::default()
        };

        let mut client = IpcClient::new(client_config);

        let result = client.connect().await;

        assert!(matches!(result, Err(IPCError::ConnectionFailed(_))));
    }

    #[tokio::test]
    async fn test_client_timeout() {
        // Start server with slow handler
        let config = crate::server::IpcServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        let mut server = IpcServer::new(config);
        server
            .register("slow", |_params| async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                Ok(serde_json::json!("done"))
            })
            .await
            .unwrap();

        let bound_addr = server.start().await.unwrap();

        // Connect client with short timeout
        let client_config = IpcClientConfig {
            server_addr: bound_addr,
            request_timeout_ms: 100,
            ..Default::default()
        };

        let mut client = IpcClient::new(client_config);
        client.connect().await.unwrap();

        let result = client
            .request("slow".to_string(), serde_json::json!(null))
            .await;

        assert!(matches!(result, Err(IPCError::Timeout(_))));

        client.disconnect().await.unwrap();
        server.shutdown().await.unwrap();
    }
}
