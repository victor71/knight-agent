//! Daemon process - Background service for Knight Agent
//!
//! The daemon runs as a background process and handles:
//! - Session management
//! - Agent runtime
//! - IPC server for TUI and session processes

use anyhow::{Context, Result};
use bootstrap::KnightAgentSystem;
use session_manager::{AgentRuntimeProxy, StreamCallback};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use crate::{ensure_dir, get_home_dir, AGENT_SUBDIRS, CONFIG_DIR};

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

        Ok(Self {
            router,
            session_manager,
            system,
        })
    }

    /// Register IPC method handlers
    pub async fn register_handlers(&self, server: &mut ipc_contract::IpcServer) {

        let router = self.router.clone();

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

                // Create a channel to receive chunks from the agent
                let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

                // Spawn task to forward chunks to StreamingContext
                tokio::spawn(async move {
                    let mut sequence = 0u64;
                    let mut total_chunks = 0;
                    while let Some(chunk) = chunk_rx.recv().await {
                        total_chunks += 1;
                        info!("[DAEMON] Sending stream chunk {}: {} chars", sequence, chunk.len());
                        let _ = stream_ctx.send_chunk(chunk, sequence, false);
                        sequence += 1;
                    }
                    info!("[DAEMON] Stream forwarding complete: {} total chunks", total_chunks);
                });

                // Create streaming callback that sends to our channel
                let stream_callback: StreamCallback = Box::new({
                    let chunk_tx = chunk_tx.clone();
                    move |chunk: String| -> bool {
                        let _ = chunk_tx.send(chunk);
                        true // Continue streaming
                    }
                });

                // Call the session manager with streaming
                info!("[DAEMON] Calling session_manager.send_message_to_session_streaming");
                let result = session_manager.send_message_to_session_streaming(session_id, content, Some(stream_callback)).await
                    .map_err(|e| ipc_contract::IPCError::InternalError(e.to_string()))?;
                info!("[DAEMON] send_message_to_session_streaming completed, response_len={}", result.len());

                Ok(serde_json::json!({ "response": result }))
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

        // create_session handler
        let session_manager = self.session_manager.clone();
        server.register("create_session", move |params: serde_json::Value| {
            let session_manager = session_manager.clone();
            Box::pin(async move {
                let name = params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                let workspace = params.get("workspace").and_then(|v| v.as_str()).unwrap_or(".").to_string();

                let mut request = session_manager::CreateSessionRequest::new(workspace);
                if let Some(name) = name {
                    request = request.name(name);
                }

                let session = session_manager.create_session(request).await
                    .map_err(|e| ipc_contract::IPCError::InternalError(e.to_string()))?;

                Ok(serde_json::json!({ "session_id": session.id }))
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

        info!("IPC method handlers registered");
    }
}

/// Run the daemon process
pub(crate) async fn run_daemon(port: u16) -> Result<()> {
    info!("Starting Knight Agent daemon on port {}...", port);

    // Ensure system configuration
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

    // Register method handlers
    state.register_handlers(&mut server).await;

    // Start the server
    info!("Starting IPC server on {}...", addr);
    let bound_addr = server.start().await
        .context("IPC server failed")?;

    info!("IPC server listening on {}", bound_addr);

    // Keep the server running
    tokio::time::sleep(std::time::Duration::MAX).await;

    Ok(())
}
