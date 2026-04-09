//! Daemon process - Background service for Knight Agent
//!
//! The daemon runs as a background process and handles:
//! - Session management
//! - Agent runtime
//! - IPC server for TUI and session processes

use anyhow::{Context, Result};
use bootstrap::KnightAgentSystem;
use session_manager::{AgentRuntimeProxy, StreamCallback};
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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

/// Daemon state
pub struct DaemonState {
    pub(crate) router: Arc<dyn router::RouterHandle>,
    pub(crate) session_manager: Arc<session_manager::SessionManagerImpl>,
    pub(crate) system: KnightAgentSystem,
}

impl DaemonState {
    /// Create a new daemon state with all components initialized
    pub async fn new() -> Result<Self> {
        info!("Initializing daemon components...");

        // Initialize global configuration first (before other modules)
        let config_dir = get_home_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(CONFIG_DIR);
        configuration::init_global_config(config_dir.clone())
            .await
            .context("Failed to initialize global configuration")?;
        info!("Global configuration initialized");

        // Initialize system
        let config = bootstrap::BootstrapConfig::default();
        let system = KnightAgentSystem::with_config(config);
        system.bootstrap().await?;
        info!("System bootstrap complete");

        // Create Router
        let router_impl = router::RouterImpl::new();
        router_impl.initialize().await?;
        let router: Arc<dyn router::RouterHandle> = Arc::new(router_impl);
        info!("Router initialized");

        // Create Agent Runtime
        let mut agent_runtime_impl = agent_runtime::AgentRuntimeImpl::new();
        agent_runtime_impl.initialize().await?;

        let agent_runtime: Arc<dyn AgentRuntimeProxy> = Arc::new(agent_runtime_impl);
        info!("Agent Runtime initialized");

        // Create Session Manager and connect with Agent Runtime
        let session_manager = Arc::new(session_manager::SessionManagerImpl::new());
        session_manager.initialize().await?;

        // Set agent runtime for session manager
        session_manager.set_agent_runtime(agent_runtime).await;
        info!("Session Manager initialized and connected to Agent Runtime");

        // Restore sessions from storage if any exist
        match session_manager.restore_sessions().await {
            Ok(_) => info!("Restored sessions from storage"),
            Err(e) => info!("No sessions to restore or restore failed: {}", e),
        }

        Ok(Self {
            router,
            session_manager,
            system,
        })
    }

    /// Register IPC method handlers
    pub async fn register_handlers(&self, server: &mut ipc_contract::IpcServer, shutdown_tx: broadcast::Sender<()>, daemon_addr: String) {

        let router = self.router.clone();
        let session_manager = self.session_manager.clone();
        let daemon_addr_clone = daemon_addr.clone();

        // handle_input handler
        server.register("handle_input", move |params: serde_json::Value| {
            let router = router.clone();
            Box::pin(async move {
                let input = params.get("input").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("default").to_string();

                let result = router.handle_input(input, session_id).await;

                // Convert HandleInputResult to JSON
                let response = serde_json::json!({
                    "response": {
                        "success": result.response.success,
                        "message": result.response.message,
                        "data": result.response.data,
                        "error": result.response.error,
                        "to_agent": result.response.to_agent,
                    },
                    "to_agent": result.to_agent,
                });

                Ok(response)
            })
        }).await;

        // send_message streaming handler
        let session_manager = self.session_manager.clone();
        server.register_streaming("send_message", move |params: serde_json::Value, stream_ctx: ipc_contract::StreamingContext| {
            let session_manager = session_manager.clone();
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();

                info!("[DAEMON] send_message streaming: session_id={}, content_len={}", session_id, content.len());

                // Create channels to receive chunks from the agent and signal completion
                let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<()>();

                // Spawn task to forward chunks to StreamingContext
                // Tokio spawn handles panics gracefully
                tokio::spawn(async move {
                    let mut sequence = 0u64;
                    let mut total_chunks = 0;
                    while let Some(chunk) = chunk_rx.recv().await {
                        total_chunks += 1;
                        info!("[DAEMON] Sending stream chunk {}: {} chars", sequence, chunk.len());
                        // Ignore send errors - client may have disconnected
                        let _ = stream_ctx.send_chunk(chunk, sequence, false);
                        sequence += 1;
                    }
                    info!("[DAEMON] Stream forwarding complete: {} total chunks", total_chunks);
                    // Signal that streaming is complete
                    let _ = done_tx.send(());
                });

                // Create streaming callback that sends to our channel
                let stream_callback: StreamCallback = Box::new({
                    let chunk_tx = chunk_tx.clone();
                    move |chunk: String| -> bool {
                        // Ignore send errors - channel may be closed
                        let _ = chunk_tx.send(chunk);
                        true // Continue streaming
                    }
                });

                // Call the session manager with streaming - wrap in error handler
                info!("[DAEMON] Calling session_manager.send_message_to_session_streaming");
                match session_manager.send_message_to_session_streaming(session_id, content, Some(stream_callback)).await {
                    Ok(result) => {
                        info!("[DAEMON] send_message_to_session_streaming completed, response_len={}", result.len());

                        // Wait for streaming chunks to be fully forwarded before returning response
                        // This is critical - don't close the connection until all chunks are sent
                        match tokio::time::timeout(tokio::time::Duration::from_secs(5), done_rx).await {
                            Ok(Ok(())) => {
                                info!("[DAEMON] All chunks forwarded successfully");
                            }
                            Ok(Err(_)) => {
                                warn!("[DAEMON] Chunk forwarding channel closed unexpectedly");
                            }
                            Err(_) => {
                                warn!("[DAEMON] Timeout waiting for chunk forwarding");
                            }
                        }

                        // Additional delay to ensure OS buffers are flushed
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        Ok(serde_json::json!({ "response": result }))
                    }
                    Err(e) => {
                        warn!("[DAEMON] send_message_to_session_streaming error: {:?}", e);
                        // Return error response but don't fail - let client handle it
                        Ok(serde_json::json!({
                            "response": format!("Error: {}", e),
                            "error": true
                        }))
                    }
                }
            })
        }).await;

        // list_sessions handler
        let session_manager = self.session_manager.clone();
        server.register("list_sessions", move |_params: serde_json::Value| {
            let session_manager = session_manager.clone();
            Box::pin(async move {
                let sessions = session_manager.list_sessions(None).await;

                let session_list: Vec<serde_json::Value> = sessions.into_iter().map(|s| {
                    serde_json::json!({
                        "id": s.id,
                        "name": s.metadata.name,
                        "status": format!("{:?}", s.status),
                        "created_at": s.created_at,
                        "message_count": s.stats.total_messages,
                    })
                }).collect();

                Ok(serde_json::json!({ "sessions": session_list }))
            })
        }).await;

        // create_session handler - spawns a dedicated session process
        let session_manager = self.session_manager.clone();
        server.register("create_session", move |params: serde_json::Value| {
            let session_manager = session_manager.clone();
            let daemon_addr = daemon_addr_clone.clone();
            Box::pin(async move {
                let name = params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                let workspace = params.get("workspace").and_then(|v| v.as_str()).unwrap_or(".").to_string();

                let mut request = session_manager::CreateSessionRequest::new(workspace);
                if let Some(name) = name {
                    request = request.name(name);
                }

                let session = session_manager.create_session(request).await
                    .map_err(|e| ipc_contract::IPCError::InternalError(e.to_string()))?;

                // Spawn dedicated session process
                let exe_path = std::env::current_exe()
                    .map_err(|e| ipc_contract::IPCError::InternalError(format!("failed to get exe path: {}", e)))?;

                let child = Command::new(&exe_path)
                    .arg("session")
                    .arg("--session-id")
                    .arg(session.id.clone())
                    .arg("--daemon-addr")
                    .arg(&daemon_addr)
                    .spawn()
                    .map_err(|e| ipc_contract::IPCError::InternalError(format!("failed to spawn session process: {}", e)))?;

                info!("Spawned session process for {} with PID: {}", session.id, child.id());

                Ok(serde_json::json!({
                    "session_id": session.id,
                    "process_id": child.id()
                }))
            })
        }).await;

        // switch_session handler
        let session_manager = self.session_manager.clone();
        server.register("switch_session", move |params: serde_json::Value| {
            let session_manager = session_manager.clone();
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");

                session_manager.use_session(session_id).await
                    .map_err(|e| ipc_contract::IPCError::InternalError(e.to_string()))?;

                Ok(serde_json::json!({ "success": true }))
            })
        }).await;

        // get_system_status handler
        let system = self.system.clone();
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

        // register_session handler (for session processes)
        server.register("register_session", move |params: serde_json::Value| {
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");

                info!("Session process registered: {}", session_id);

                Ok(serde_json::json!({
                    "success": true,
                    "message": format!("Session {} registered", session_id),
                }))
            })
        }).await;

        // heartbeat handler (for session processes)
        server.register("heartbeat", move |params: serde_json::Value| {
            Box::pin(async move {
                let session_id = params.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                let _timestamp = params.get("timestamp").and_then(|v| v.as_str());

                // Update session last activity timestamp
                info!("Heartbeat from session: {}", session_id);

                Ok(serde_json::json!({ "success": true }))
            })
        }).await;

        // shutdown handler - save all sessions before shutdown
        let session_manager = self.session_manager.clone();
        let shutdown_tx = shutdown_tx.clone();
        server.register("shutdown", move |_params: serde_json::Value| {
            let session_manager = session_manager.clone();
            let shutdown_tx = shutdown_tx.clone();
            Box::pin(async move {
                info!("Shutdown request received, saving all sessions...");
                match session_manager.save_all_sessions().await {
                    Ok(saved_paths) => {
                        info!("Saved {} sessions before shutdown", saved_paths.len());
                    }
                    Err(e) => {
                        warn!("Failed to save sessions during shutdown: {}", e);
                    }
                }
                // Signal shutdown to the server loop (ignore result - receiver may already be dropped)
                let _ = shutdown_tx.send(());
                Ok(serde_json::json!({ "success": true }))
            })
        }).await;

        info!("IPC method handlers registered");
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
