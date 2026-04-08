//! Knight Agent - Main Entry Point
//!
//! This binary supports multiple execution modes:
//! - In-process mode (default or --in-process): All components in one process
//! - Daemon mode (knight-agent daemon): Run as background service
//! - Session mode (knight-agent session): Run a dedicated session process

use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;
use tui::{DaemonClient, IpcDaemonClient};

use daemon_manager::connect_to_daemon;

mod args;
mod in_process;
mod daemon;
mod session;
mod daemon_manager;

use args::Args;

/// Default configuration directory name
const CONFIG_DIR: &str = ".knight-agent";

/// Subdirectories to create under .knight-agent
const AGENT_SUBDIRS: &[&str] = &["sessions", "logs", "skills", "commands"];

/// Get the user's home directory for config storage
pub(crate) fn get_home_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .context("Failed to get USERPROFILE environment variable")
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .context("Failed to get HOME environment variable")
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .context("Failed to get HOME environment variable")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .context("Failed to get HOME environment variable")
    }
}

/// Ensure a directory exists, creating it if necessary
pub(crate) fn ensure_dir(path: &Path, name: &str) -> Result<bool> {
    if path.exists() {
        if path.is_dir() {
            info!("{} directory exists: {}", name, path.display());
            Ok(true)
        } else {
            tracing::warn!("{} path exists but is not a directory: {}", name, path.display());
            Ok(false)
        }
    } else {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create {} directory: {}", name, path.display()))?;
        info!("Created {} directory: {}", name, path.display());
        Ok(true)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Dispatch to appropriate mode
    if args.is_in_process_mode() {
        // Run in single-process mode (default or --in-process)
        in_process::run_in_process().await
    } else if args.is_daemon_mode() {
        // Run as daemon
        if let Some(args::Command::Daemon { port }) = args.command {
            daemon::run_daemon(port).await
        } else {
            unreachable!()
        }
    } else if args.is_session_mode() {
        // Run as session process
        if let Some(args::Command::Session { session_id, daemon_addr }) = args.command {
            session::run_session(session_id, daemon_addr).await
        } else {
            unreachable!()
        }
    } else {
        // Default mode: Try IPC mode first, fallback to in-process
        match run_tui_with_ipc().await {
            Ok(_) => Ok(()),
            Err(e) => {
                info!("IPC mode failed: {}, falling back to in-process mode", e);
                in_process::run_in_process().await
            }
        }
    }
}

/// Run TUI with IPC connection to daemon
async fn run_tui_with_ipc() -> Result<()> {
    info!("Starting TUI with IPC connection to daemon...");

    // Connect to daemon (will spawn if needed)
    let daemon_addr = connect_to_daemon().await?;

    // Create IPC daemon client
    let daemon_client: Arc<dyn DaemonClient> = Arc::new(
        IpcDaemonClient::new(daemon_addr).await?
    );

    // Run TUI with IPC client
    // TODO: This needs to be integrated with the actual TUI startup
    // For now, this is a placeholder
    info!("TUI with IPC not yet fully implemented");

    Ok(())
}
