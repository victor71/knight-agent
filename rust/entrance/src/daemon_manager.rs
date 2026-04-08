//! Daemon management utilities
//!
//! Functions for spawning, connecting to, and managing the daemon process.

use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

const DEFAULT_DAEMON_PORT: u16 = 8080;
const DEFAULT_DAEMON_ADDR: &str = "127.0.0.1:8080";

/// Check if daemon is running on the specified address
pub async fn is_daemon_running(addr: &str) -> bool {
    // Try to connect to the daemon
    match tokio::net::TcpStream::connect(addr).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Spawn the daemon process
pub fn spawn_daemon() -> Result<()> {
    info!("Spawning daemon process...");

    let exe_path = std::env::current_exe()
        .context("Failed to get current executable path")?;

    let mut child = Command::new(&exe_path)
        .arg("daemon")
        .arg("--port")
        .arg(DEFAULT_DAEMON_PORT.to_string())
        .spawn()
        .context("Failed to spawn daemon process")?;

    info!("Daemon spawned with PID: {:?}", child.id());

    // Don't wait for the child - let it run in background
    let _ = child.try_wait();

    Ok(())
}

/// Connect to daemon with retry logic
pub async fn connect_to_daemon() -> Result<String> {
    let addr = DEFAULT_DAEMON_ADDR.to_string();
    let max_retries = 10;
    let retry_delay = Duration::from_millis(500);

    // First check if daemon is already running
    if is_daemon_running(&addr).await {
        info!("Daemon is already running on {}", addr);
        return Ok(addr);
    }

    // Try to spawn the daemon
    info!("Daemon not running, attempting to spawn...");
    spawn_daemon()?;

    // Wait for daemon to start
    for i in 0..max_retries {
        sleep(retry_delay).await;

        if is_daemon_running(&addr).await {
            info!("Successfully connected to daemon after {} attempts", i + 1);
            return Ok(addr);
        }

        info!("Waiting for daemon to start... ({}/{})", i + 1, max_retries);
    }

    Err(anyhow::anyhow!("Failed to connect to daemon after {} attempts", max_retries))
}
