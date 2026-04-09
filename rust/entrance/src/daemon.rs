//! Daemon process - Background service for Knight Agent (IPC Broker)
//!
//! The daemon runs as a background process and acts as an IPC broker:
//! - Relays messages between TUI and session processes
//! - Maintains session registry (session_id -> port mapping)
//! - Does NOT own LLM stack (session processes own their own LLM)

use anyhow::{Context, Result};
use bootstrap::KnightAgentSystem;
use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{info, warn, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::{ensure_dir, get_home_dir, AGENT_SUBDIRS, CONFIG_DIR};

/// Session-based rotating log writer state
pub(crate) struct SessionLogWriter {
    log_dir: PathBuf,
    current_session_id: Mutex<Option<String>>,
    current_file: Mutex<Option<PathBuf>>,
    current_size: Mutex<u64>,
    file_index: Mutex<u32>,
    max_file_size: u64,
}

impl SessionLogWriter {
    pub(crate) fn new(log_dir: PathBuf, max_file_size: u64) -> Self {
        Self {
            log_dir,
            current_session_id: Mutex::new(None),
            current_file: Mutex::new(None),
            current_size: Mutex::new(0),
            file_index: Mutex::new(0),
            max_file_size,
        }
    }

    fn generate_log_path(&self, session_id: &str, index: u32) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = if index == 0 {
            format!("daemon_{}.log", timestamp)
        } else {
            format!("daemon_{}_{}.log", timestamp, index)
        };
        self.log_dir.join(filename)
    }

    pub(crate) fn set_session(&self, session_id: String) -> Result<()> {
        let mut current_session = self.current_session_id.lock().unwrap();
        if current_session.as_ref() == Some(&session_id) {
            return Ok(());
        }
        *current_session = Some(session_id.clone());
        *self.file_index.lock().unwrap() = 0;
        *self.current_size.lock().unwrap() = 0;

        let log_path = self.generate_log_path(&session_id, 0);
        std::fs::write(&log_path, "").context("Failed to create log file")?;
        *self.current_file.lock().unwrap() = Some(log_path);

        info!("Created new log file for daemon session: {}", session_id);
        Ok(())
    }

    fn check_rotation(&self) -> Result<()> {
        let current_session = self.current_session_id.lock().unwrap();
        let session_id = match current_session.as_ref() {
            Some(id) => id,
            None => return Ok(()),
        };

        let current_file = self.current_file.lock().unwrap();
        let file_path = match current_file.as_ref() {
            Some(p) => p,
            None => return Ok(()),
        };

        let metadata = std::fs::metadata(file_path)?;
        let size = metadata.len();
        *self.current_size.lock().unwrap() = size;

        if size >= self.max_file_size {
            drop(current_file);
            let mut index = *self.file_index.lock().unwrap() + 1;
            *self.file_index.lock().unwrap() = index;

            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let mut new_path;
            loop {
                new_path = self.log_dir.join(format!("daemon_{}_{}.log", timestamp, index));
                if !new_path.exists() {
                    break;
                }
                index += 1;
            }

            std::fs::write(&new_path, "")?;
            *self.current_file.lock().unwrap() = Some(new_path.clone());
            *self.current_size.lock().unwrap() = 0;

            info!("Rotated log file to: {}", new_path.display());
        }

        Ok(())
    }

    fn write_data(&self, buf: &[u8]) -> std::io::Result<usize> {
        if let Err(e) = self.check_rotation() {
            eprintln!("Error checking log rotation: {}", e);
        }

        let current_file = self.current_file.lock().unwrap();
        let file_path = match current_file.as_ref() {
            Some(p) => p,
            None => return Ok(0),
        };

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        let result = IoWrite::write(&mut file, buf);

        if result.is_ok() {
            let mut size = self.current_size.lock().unwrap();
            *size += buf.len() as u64;
        }

        result
    }

    fn flush_data(&self) -> std::io::Result<()> {
        let current_file = self.current_file.lock().unwrap();
        if let Some(file_path) = current_file.as_ref() {
            let mut file = std::fs::OpenOptions::new().append(true).open(file_path)?;
            IoWrite::flush(&mut file)
        } else {
            Ok(())
        }
    }
}

struct LogWriter(Arc<Mutex<SessionLogWriter>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write_data(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush_data()
    }
}

/// Initialize logging for daemon
fn init_logging(log_dir: &PathBuf, max_file_size_mb: u64) -> Result<(WorkerGuard, Arc<Mutex<SessionLogWriter>>)> {
    let max_file_size = max_file_size_mb * 1024 * 1024;
    let log_writer = Arc::new(Mutex::new(SessionLogWriter::new(log_dir.clone(), max_file_size)));
    log_writer.lock().unwrap().set_session("daemon".to_string())?;

    let (file_writer, guard) = tracing_appender::non_blocking(LogWriter(log_writer.clone()));

    let subscriber = tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_writer(file_writer)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok((guard, log_writer))
}

/// Session entry in the registry
#[derive(Clone)]
pub struct SessionEntry {
    pub session_id: String,
    pub pid: u32,
    pub port: u16,
}

/// Daemon state - IPC broker only, does not own LLM stack
pub struct DaemonState {
    pub(crate) system: KnightAgentSystem,
    /// Session registry - maps session_id to session process info
    pub(crate) session_registry: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, SessionEntry>>>,
}

impl DaemonState {
    /// Create a new daemon state - IPC broker only, no LLM stack
    pub async fn new() -> Result<Self> {
        info!("Initializing daemon (IPC broker mode)...");

        // Initialize global configuration
        let config_dir = get_home_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(CONFIG_DIR);
        configuration::init_global_config(config_dir.clone())
            .await
            .context("Failed to initialize global configuration")?;
        info!("Global configuration initialized");

        // Initialize system in daemon mode (IPC broker, no LLM stack)
        let config = bootstrap::BootstrapConfig {
            mode: bootstrap::BootstrapMode::Daemon,
            ..Default::default()
        };
        let system = KnightAgentSystem::with_config(config);
        system.bootstrap().await?;
        info!("System bootstrap complete");

        // Create session registry
        let session_registry = std::sync::Arc::new(
            std::sync::Mutex::new(std::collections::HashMap::new())
        );
        info!("Session registry initialized");

        Ok(Self {
            system,
            session_registry,
        })
    }

    /// Register IPC method handlers
    pub async fn register_handlers(&self, server: &mut ipc_contract::IpcServer, shutdown_tx: broadcast::Sender<()>, daemon_addr: String) {
        let session_registry = self.session_registry.clone();
        let system = self.system.clone();
        let daemon_addr_clone = daemon_addr.clone();

        // handle_input handler - relay to session process
        let registry_for_input = session_registry.clone();
        server.register("handle_input", move |params: serde_json::Value| {
            let session_registry = registry_for_input.clone();
            Box::pin(async move {
                let input = params.get("input").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("default").to_string();

                // Look up session port
                let session_entry = {
                    let registry = session_registry.lock().unwrap();
                    registry.get(&session_id).cloned()
                };

                if let Some(entry) = session_entry {
                    // Relay to session process via IPC
                    let session_addr = format!("127.0.0.1:{}", entry.port);
                    let socket_addr: std::net::SocketAddr = session_addr.parse()
                        .map_err(|e| ipc_contract::IPCError::InternalError(format!("Invalid session addr: {}", e)))?;

                    let client_config = ipc_contract::IpcClientConfig {
                        server_addr: socket_addr,
                        connect_timeout_ms: 5000,
                        request_timeout_ms: 30000,
                        event_channel_size: 100,
                    };

                    let mut session_client = ipc_contract::IpcClient::new(client_config);
                    if let Err(e) = session_client.connect().await {
                        return Err(ipc_contract::IPCError::InternalError(format!("Failed to connect to session: {}", e)));
                    }

                    let relay_params = serde_json::json!({
                        "input": input,
                        "session_id": session_id,
                    });

                    let response = session_client.request("handle_input".to_string(), relay_params).await
                        .map_err(|e| ipc_contract::IPCError::InternalError(format!("handle_input relay failed: {}", e)))?;

                    session_client.disconnect().await;
                    Ok(response)
                } else {
                    // No session process found - return error
                    Ok(serde_json::json!({
                        "response": {
                            "success": false,
                            "message": "",
                            "data": null,
                            "error": format!("Session {} not found or session process not running", session_id),
                            "to_agent": false,
                        },
                        "to_agent": false,
                    }))
                }
            })
        }).await;

        // send_message streaming handler - relay to session process
        let registry_for_send = session_registry.clone();
        server.register_streaming("send_message", move |params: serde_json::Value, stream_ctx: ipc_contract::StreamingContext| {
            let session_registry = registry_for_send.clone();
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();

                info!("[DAEMON] send_message relay: session_id={}, content_len={}", session_id, content.len());

                // Look up session port
                let session_entry = {
                    let registry = session_registry.lock().unwrap();
                    registry.get(session_id).cloned()
                };

                if let Some(entry) = session_entry {
                    // Connect to session process
                    let session_addr = format!("127.0.0.1:{}", entry.port);
                    let socket_addr: std::net::SocketAddr = session_addr.parse()
                        .map_err(|e| ipc_contract::IPCError::InternalError(format!("Invalid session addr: {}", e)))?;

                    let client_config = ipc_contract::IpcClientConfig {
                        server_addr: socket_addr,
                        connect_timeout_ms: 5000,
                        request_timeout_ms: 120000, // Longer timeout for LLM calls
                        event_channel_size: 100,
                    };

                    let mut session_client = ipc_contract::IpcClient::new(client_config);
                    if let Err(e) = session_client.connect().await {
                        return Ok(serde_json::json!({
                            "response": format!("Failed to connect to session: {}", e),
                            "error": true
                        }));
                    }

                    let relay_params = serde_json::json!({
                        "content": content,
                    });

                    // Relay streaming request to session
                    match session_client.request_streaming("send_message".to_string(), relay_params).await {
                        Ok((mut chunk_rx, response_rx)) => {
                            // Forward chunks to the original caller
                            let mut sequence = 0u64;
                            while let Some(chunk_msg) = chunk_rx.recv().await {
                                let _ = stream_ctx.send_chunk(chunk_msg.chunk, sequence, false);
                                sequence += 1;
                            }

                            // Wait for final response
                            match response_rx.await {
                                Ok(response) => {
                                    session_client.disconnect().await;
                                    // Extract result from ResponseMessage
                                    Ok(response.result.unwrap_or(serde_json::json!({
                                        "response": ""
                                    })))
                                }
                                Err(e) => {
                                    session_client.disconnect().await;
                                    Ok(serde_json::json!({
                                        "response": format!("Error: {}", e),
                                        "error": true
                                    }))
                                }
                            }
                        }
                        Err(e) => {
                            session_client.disconnect().await;
                            Ok(serde_json::json!({
                                "response": format!("Failed to relay to session: {}", e),
                                "error": true
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "response": format!("Session {} not found or session process not running", session_id),
                        "error": true
                    }))
                }
            })
        }).await;

        // list_sessions handler - query all registered sessions
        let registry_for_list = session_registry.clone();
        server.register("list_sessions", move |_params: serde_json::Value| {
            let session_registry = registry_for_list.clone();
            Box::pin(async move {
                let sessions = {
                    let registry = session_registry.lock().unwrap();
                    registry.values().map(|e| serde_json::json!({
                        "id": e.session_id,
                        "name": e.session_id,
                        "status": "Active",
                        "created_at": "",
                        "message_count": 0,
                    })).collect::<Vec<_>>()
                };

                Ok(serde_json::json!({ "sessions": sessions }))
            })
        }).await;

        // create_session handler - spawns a dedicated session process
        let registry_for_create = session_registry.clone();
        server.register("create_session", move |params: serde_json::Value| {
            let session_registry = registry_for_create.clone();
            let daemon_addr = daemon_addr_clone.clone();
            Box::pin(async move {
                let name = params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                let workspace = params.get("workspace").and_then(|v| v.as_str()).unwrap_or(".").to_string();

                // Generate session ID
                let session_id = format!("sess_{}", uuid::Uuid::new_v4());

                // Spawn dedicated session process
                let exe_path = std::env::current_exe()
                    .map_err(|e| ipc_contract::IPCError::InternalError(format!("failed to get exe path: {}", e)))?;

                let child = Command::new(&exe_path)
                    .arg("session")
                    .arg("--session-id")
                    .arg(session_id.clone())
                    .arg("--daemon-addr")
                    .arg(&daemon_addr)
                    .spawn()
                    .map_err(|e| ipc_contract::IPCError::InternalError(format!("failed to spawn session process: {}", e)))?;

                info!("Spawned session process for {} with PID: {}", session_id, child.id());

                // Add to session registry (will be updated when session registers with port)
                {
                    let mut registry = session_registry.lock().unwrap();
                    registry.insert(session_id.clone(), SessionEntry {
                        session_id: session_id.clone(),
                        pid: child.id(),
                        port: 0, // Will be updated when session registers
                    });
                }

                Ok(serde_json::json!({
                    "session_id": session_id,
                    "process_id": child.id()
                }))
            })
        }).await;

        // switch_session handler - just acknowledge (session selection is client-side)
        server.register("switch_session", move |params: serde_json::Value| {
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                info!("Switch session request: {}", session_id);
                Ok(serde_json::json!({ "success": true }))
            })
        }).await;

        // get_system_status handler
        let system = system.clone();
        server.register("get_system_status", move |_params: serde_json::Value| {
            let system = system.clone();
            Box::pin(async move {
                let status = system.status().await;

                Ok(serde_json::json!({
                    "stage": status.stage,
                    "initialized": status.initialized,
                    "ready": status.ready,
                    "module_count": status.module_count,
                    "initialized_count": status.initialized_count,
                }))
            })
        }).await;

        // register_session handler (for session processes) - update registry with port
        let registry_for_register = session_registry.clone();
        server.register("register_session", move |params: serde_json::Value| {
            let session_registry = registry_for_register.clone();
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                let pid = params.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

                info!("Session process registered: {} on port {} (PID: {})", session_id, port, pid);

                // Update registry with port
                {
                    let mut registry = session_registry.lock().unwrap();
                    if let Some(entry) = registry.get_mut(session_id) {
                        entry.port = port;
                    }
                }

                Ok(serde_json::json!({
                    "success": true,
                    "message": format!("Session {} registered on port {}", session_id, port),
                }))
            })
        }).await;

        // heartbeat handler (for session processes)
        server.register("heartbeat", move |params: serde_json::Value| {
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                let _timestamp = params.get("timestamp").and_then(|v| v.as_str());

                info!("Heartbeat from session: {}", session_id);

                Ok(serde_json::json!({ "success": true }))
            })
        }).await;

        // shutdown handler - signal shutdown
        let shutdown_tx = shutdown_tx.clone();
        server.register("shutdown", move |_params: serde_json::Value| {
            let shutdown_tx = shutdown_tx.clone();
            Box::pin(async move {
                info!("Shutdown request received");
                let _ = shutdown_tx.send(());
                Ok(serde_json::json!({ "success": true }))
            })
        }).await;

        info!("IPC method handlers registered (IPC broker mode)");
    }
}

/// Run the daemon process
pub(crate) async fn run_daemon(port: u16) -> Result<()> {
    // Ensure system configuration first
    let home_dir = get_home_dir()?;
    let config_dir = home_dir.join(CONFIG_DIR);
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .context("Failed to create .knight-agent directory")?;
    }

    // Create subdirectories
    for subdir in AGENT_SUBDIRS {
        let dir_path = config_dir.join(subdir);
        ensure_dir(&dir_path, subdir)?;
    }

    // Initialize logging before anything else
    let log_dir = config_dir.join("logs");
    std::fs::create_dir_all(&log_dir).context("Failed to create logs directory")?;
    let (_guard, _log_writer) = init_logging(&log_dir, 10)?;  // 10MB max file size

    info!("Starting Knight Agent daemon on port {}...", port);

    // Initialize daemon state
    let state = DaemonState::new().await?;

    // Create IPC server config
    let addr = format!("127.0.0.1:{}", port);
    let server_addr: std::net::SocketAddr = addr.parse()
        .context("Invalid server address")?;

    let server_config = ipc_contract::IpcServerConfig {
        bind_addr: server_addr,
        max_connections: 100,
        request_queue_size: 100,
    };

    let mut server = ipc_contract::IpcServer::new(server_config);

    // Create broadcast channel for graceful shutdown
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

    // Start the server first to get the bound address
    info!("Starting IPC server on {}...", addr);
    let bound_addr = server.start().await
        .context("IPC server failed")?;

    info!("IPC server listening on {}", bound_addr);

    // Register method handlers (now we have the actual bound address)
    state.register_handlers(&mut server, shutdown_tx, addr).await;

    info!("IPC server listening on {}", bound_addr);

    // Wait for shutdown signal
    let _ = shutdown_rx.recv().await;
    info!("Shutdown signal received, stopping server...");

    // Shutdown the server gracefully
    server.shutdown().await.context("Server shutdown failed")?;
    info!("Daemon shutdown complete");

    Ok(())
}
