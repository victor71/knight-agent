//! Session process - Dedicated process for a single agent session
//!
//! A session process is spawned by the daemon to handle a single session's
//! agent interactions. It connects to the daemon via IPC.

use anyhow::Result;
use tracing::info;

/// Run a session process
pub(crate) async fn run_session(session_id: String, daemon_addr: String) -> Result<()> {
    info!("Starting session process for session: {}", session_id);
    info!("Connecting to daemon at: {}", daemon_addr);

    // TODO: Phase 5 - Initialize session components
    // - Connect to daemon via IPC
    // - Initialize agent for this session
    // - Handle incoming messages from daemon

    info!("Session process started (placeholder - will be implemented in Phase 5)");

    // Keep the session process running
    tokio::time::sleep(std::time::Duration::MAX).await;

    Ok(())
}
