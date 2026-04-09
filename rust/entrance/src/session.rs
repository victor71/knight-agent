//! Session process - Dedicated process for a single agent session
//!
//! A session process is spawned by the daemon to handle a single session's
//! agent interactions. It owns its own LLM stack (Router + AgentRuntime + SessionManager)
//! and connects to the daemon via IPC for registration and message relay.

use anyhow::{Context, Result};
use bootstrap::{BootstrapConfig, BootstrapMode, KnightAgentSystem};
use session_manager::StreamCallback;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, warn, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::{get_home_dir, AGENT_SUBDIRS, CONFIG_DIR};

/// Session-based rotating log writer state (copied from daemon.rs pattern)
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
            format!("session_{}_{}.log", session_id, timestamp)
        } else {
            format!("session_{}_{}_{}.log", session_id, timestamp, index)
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

        info!("Created new log file for session: {}", session_id);
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
            let session_id = current_session.clone().unwrap_or_default();
            drop(current_file);
            let mut index = *self.file_index.lock().unwrap() + 1;
            *self.file_index.lock().unwrap() = index;

            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let mut new_path;
            loop {
                new_path = self.log_dir.join(format!("session_{}_{}_{}.log", session_id, timestamp, index));
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

        let result = std::io::Write::write(&mut file, buf);

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
            std::io::Write::flush(&mut file)
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

/// Initialize logging for session process
fn init_logging(session_id: &str, log_dir: &PathBuf, max_file_size_mb: u64) -> Result<(WorkerGuard, Arc<Mutex<SessionLogWriter>>)> {
    let max_file_size = max_file_size_mb * 1024 * 1024;
    let log_writer = Arc::new(Mutex::new(SessionLogWriter::new(log_dir.clone(), max_file_size)));
    log_writer.lock().unwrap().set_session(session_id.to_string())?;

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

/// Session process state - owns LLM stack for this session
pub(crate) struct SessionState {
    pub session_id: String,
    pub router: Arc<dyn router::RouterHandle>,
    pub session_manager: Arc<session_manager::SessionManagerImpl>,
}

/// Run a session process
pub(crate) async fn run_session(session_id: String, daemon_addr: String) -> Result<()> {
    // Initialize logging for session process
    let home_dir = get_home_dir()?;
    let config_dir = home_dir.join(CONFIG_DIR);
    let log_dir = config_dir.join("logs");

    // Ensure log directory exists
    std::fs::create_dir_all(&log_dir).context("Failed to create logs directory")?;

    // Initialize logging with session ID
    let (_guard, _log_writer) = init_logging(&session_id, &log_dir, 10)?;  // 10MB max file size

    info!("Starting session process for session: {}", session_id);
    info!("Connecting to daemon at: {}", daemon_addr);

    // Parse daemon address
    let socket_addr: std::net::SocketAddr = daemon_addr.parse()
        .context("Invalid daemon address")?;

    // Create IPC client to connect to daemon
    let config = ipc_contract::IpcClientConfig {
        server_addr: socket_addr,
        connect_timeout_ms: 5000,
        request_timeout_ms: 30000,
        event_channel_size: 100,
    };

    let mut ipc_client = ipc_contract::IpcClient::new(config);

    // Connect to daemon
    ipc_client.connect()
        .await
        .context("Failed to connect to daemon")?;

    info!("Connected to daemon");

    // Initialize global configuration
    configuration::init_global_config(config_dir.clone())
        .await
        .context("Failed to initialize global configuration")?;
    info!("Global configuration initialized");

    // Initialize system bootstrap with Session mode
    // This runs the 8-stage initialization for session-specific modules
    let bootstrap_config = BootstrapConfig {
        mode: BootstrapMode::Session,
        ..Default::default()
    };
    let system = KnightAgentSystem::with_config(bootstrap_config);
    system.bootstrap().await
        .context("Failed to bootstrap session system")?;
    info!("System bootstrap complete (Session mode)");

    // Initialize LLM Router
    let router = Arc::new(router::RouterImpl::new());
    router.initialize().await?;
    info!("LLM Router initialized");

    // Initialize Agent Runtime
    let mut agent_runtime_impl = agent_runtime::AgentRuntimeImpl::new();
    agent_runtime_impl.initialize().await?;
    let agent_runtime: Arc<dyn session_manager::AgentRuntimeProxy> = Arc::new(agent_runtime_impl);
    info!("Agent Runtime initialized");

    // Initialize Session Manager and connect with Agent Runtime
    let session_manager = Arc::new(session_manager::SessionManagerImpl::new());
    session_manager.initialize().await?;
    session_manager.set_agent_runtime(agent_runtime).await;
    info!("Session Manager initialized and connected to Agent Runtime");

    // Restore session state if it exists
    match session_manager.restore_sessions().await {
        Ok(_) => info!("Restored sessions from storage"),
        Err(e) => info!("No sessions to restore or restore failed: {}", e),
    }

    // Create session state
    let state = Arc::new(SessionState {
        session_id: session_id.clone(),
        router: router.clone(),
        session_manager: session_manager.clone(),
    });

    // Create IPC server for receiving messages from daemon
    // Use port 0 to let OS assign a free port
    let server_addr: std::net::SocketAddr = "127.0.0.1:0".parse()
        .context("Invalid server address")?;

    let server_config = ipc_contract::IpcServerConfig {
        bind_addr: server_addr,
        max_connections: 10,
        request_queue_size: 50,
    };

    let mut server = ipc_contract::IpcServer::new(server_config);

    // Start the IPC server
    let bound_addr = server.start().await
        .context("Failed to start IPC server")?;
    info!("Session IPC server listening on {}", bound_addr);

    // Extract port from bound address
    let session_port = bound_addr.port();
    info!("Session IPC server port: {}", session_port);

    // Register this session with the daemon, including the port for IPC
    let register_params = serde_json::json!({
        "session_id": session_id,
        "port": session_port,
    });

    let _response = ipc_client.request("register_session".to_string(), register_params).await
        .context("Failed to register session with daemon")?;

    info!("Session {} registered with daemon on port {}", session_id, session_port);

    // Create shutdown signal
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    // Register IPC handlers for this session
    register_session_handlers(&mut server, state.clone(), shutdown_tx).await;

    // Main session loop - keep the process alive and handle IPC
    loop {
        tokio::select! {
            // Send heartbeat every 30 seconds
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                let heartbeat_params = serde_json::json!({
                    "session_id": session_id,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                if let Err(e) = ipc_client.request("heartbeat".to_string(), heartbeat_params).await {
                    info!("Heartbeat failed (daemon may have shut down): {}", e);
                    break;
                }
            }
            // Handle shutdown signal
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received");
                break;
            }
        }
    }

    // Shutdown IPC server
    server.shutdown().await.context("Server shutdown failed")?;

    // Save session state before exit
    info!("Saving session state before exit...");
    if let Err(e) = session_manager.save_all_sessions().await {
        warn!("Failed to save session state: {}", e);
    }

    info!("Session process exiting");
    Ok(())
}

/// Register IPC handlers for session process
async fn register_session_handlers(
    server: &mut ipc_contract::IpcServer,
    state: Arc<SessionState>,
    shutdown_tx: broadcast::Sender<()>,
) {
    let session_manager = state.session_manager.clone();
    let router = state.router.clone();
    let session_id = state.session_id.clone(); // Clone once for use in closures

    // send_message handler - process LLM message in this session's context
    let session_id_for_send = session_id.clone();
    server.register_streaming("send_message", move |params: serde_json::Value, stream_ctx: ipc_contract::StreamingContext| {
        let session_manager = session_manager.clone();
        let session_id = session_id_for_send.clone();
        Box::pin(async move {
            let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();

            info!("[SESSION] send_message streaming: content_len={}", content.len());

            // Create channels to receive chunks from the agent and signal completion
            let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<()>();

            // Spawn task to forward chunks to StreamingContext
            tokio::spawn(async move {
                let mut sequence = 0u64;
                let mut total_chunks = 0;
                while let Some(chunk) = chunk_rx.recv().await {
                    total_chunks += 1;
                    info!("[SESSION] Sending stream chunk {}: {} chars", sequence, chunk.len());
                    let _ = stream_ctx.send_chunk(chunk, sequence, false);
                    sequence += 1;
                }
                info!("[SESSION] Stream forwarding complete: {} total chunks", total_chunks);
                let _ = done_tx.send(());
            });

            // Create streaming callback
            let stream_callback: StreamCallback = Box::new({
                let chunk_tx = chunk_tx.clone();
                move |chunk: String| -> bool {
                    let _ = chunk_tx.send(chunk);
                    true
                }
            });

            // Call session manager with this session's context
            match session_manager.send_message_to_session_streaming(&session_id, content, Some(stream_callback)).await {
                Ok(result) => {
                    info!("[SESSION] send_message_to_session_streaming completed, response_len={}", result.len());

                    // Wait for streaming chunks to be fully forwarded
                    match tokio::time::timeout(tokio::time::Duration::from_secs(5), done_rx).await {
                        Ok(Ok(())) => info!("[SESSION] All chunks forwarded successfully"),
                        Ok(Err(_)) => warn!("[SESSION] Chunk forwarding channel closed unexpectedly"),
                        Err(_) => warn!("[SESSION] Timeout waiting for chunk forwarding"),
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    Ok(serde_json::json!({ "response": result }))
                }
                Err(e) => {
                    warn!("[SESSION] send_message_to_session_streaming error: {:?}", e);
                    Ok(serde_json::json!({
                        "response": format!("Error: {}", e),
                        "error": true
                    }))
                }
            }
        })
    }).await;

    // handle_input handler - route input through router
    let session_id_for_input = session_id.clone();
    server.register("handle_input", move |params: serde_json::Value| {
        let router = router.clone();
        let session_id = session_id_for_input.clone();
        Box::pin(async move {
            let input = params.get("input").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let result = router.handle_input(input, session_id).await;

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

    // shutdown handler
    let shutdown_tx_clone = shutdown_tx.clone();
    server.register("shutdown", move |_params: serde_json::Value| {
        let shutdown_tx = shutdown_tx_clone.clone();
        Box::pin(async move {
            info!("Shutdown request received in session");
            let _ = shutdown_tx.send(());
            Ok(serde_json::json!({ "success": true }))
        })
    }).await;

    info!("Session IPC handlers registered");
}
