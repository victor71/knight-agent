//! Session process - Dedicated process for a single agent session
//!
//! A session process is spawned by the daemon to handle a single session's
//! agent interactions. It connects to the daemon via IPC.

use anyhow::{Context, Result};
use std::time::Duration;
use tracing::info;

/// Run a session process
pub(crate) async fn run_session(session_id: String, daemon_addr: String) -> Result<()> {
    info!("Starting session process for session: {}", session_id);
    info!("Connecting to daemon at: {}", daemon_addr);

    // Parse daemon address
    let socket_addr: std::net::SocketAddr = daemon_addr.parse()
        .context("Invalid daemon address")?;

    // Create IPC client
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

    // Register this session with the daemon
    let register_params = serde_json::json!({
        "session_id": session_id,
    });

    let _response = ipc_client.request("register_session".to_string(), register_params).await
        .context("Failed to register session with daemon")?;

    info!("Session {} registered with daemon", session_id);

    // Main session loop - keep the process alive
    loop {
        // Send heartbeat every 30 seconds
        tokio::time::sleep(Duration::from_secs(30)).await;

        let heartbeat_params = serde_json::json!({
            "session_id": session_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        if let Err(e) = ipc_client.request("heartbeat".to_string(), heartbeat_params).await {
            info!("Heartbeat failed (daemon may have shut down): {}", e);
            break;
        }
    }

    info!("Session process exiting");
    Ok(())
}
