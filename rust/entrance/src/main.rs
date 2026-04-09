//! Knight Agent - Main Entry Point
//!
//! This binary supports multiple execution modes:
//! - In-process mode (default or --in-process): All components in one process
//! - Daemon mode (knight-agent daemon): Run as background service
//! - Session mode (knight-agent session): Run a dedicated session process

use anyhow::{Context, Result};
use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;
use tui::{DaemonClient, IpcDaemonClient, SystemStatusSnapshot, run_tui};

use daemon_manager::connect_to_daemon;

mod args;
mod in_process;
mod daemon;
mod session;
mod daemon_manager;

/// Simple log writer for TUI that writes to a rotating log file
pub(crate) struct TuiLogWriter {
    log_path: PathBuf,
    current_size: Mutex<u64>,
    max_file_size: u64,
}

impl TuiLogWriter {
    pub(crate) fn new(log_dir: &PathBuf, max_file_size_mb: u64) -> Result<Self> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("tui_{}.log", timestamp);
        let log_path = log_dir.join(filename);
        std::fs::write(&log_path, "").context("Failed to create TUI log file")?;

        Ok(Self {
            log_path,
            current_size: Mutex::new(0),
            max_file_size: max_file_size_mb * 1024 * 1024,
        })
    }

    fn write_data(&self, buf: &[u8]) -> std::io::Result<usize> {
        let mut size = self.current_size.lock().unwrap();

        // Check rotation
        if *size >= self.max_file_size {
            drop(size);
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let new_path = self.log_path.parent().unwrap().join(format!("tui_{}.log", timestamp));
            std::fs::write(&new_path, "")?;
            // Note: We'd need to update log_path but this is a simple implementation
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let result = IoWrite::write(&mut file, buf);
        if result.is_ok() {
            *self.current_size.lock().unwrap() += buf.len() as u64;
        }
        result
    }

    fn flush(&self) -> std::io::Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.log_path)?;
        IoWrite::flush(&mut file)
    }
}

struct LogWriter(Arc<Mutex<TuiLogWriter>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write_data(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// Initialize logging for TUI process
fn init_tui_logging(log_dir: &PathBuf, max_file_size_mb: u64) -> Result<(WorkerGuard, Arc<Mutex<TuiLogWriter>>)> {
    let log_writer = Arc::new(Mutex::new(TuiLogWriter::new(log_dir, max_file_size_mb)?));

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
        // Run in single-process mode (only when explicitly specified)
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
        // Default mode (IPC): Try IPC mode first, fallback to in-process
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
    // Initialize TUI logging first
    let home_dir = get_home_dir()?;
    let config_dir = home_dir.join(CONFIG_DIR);
    let log_dir = config_dir.join("logs");
    std::fs::create_dir_all(&log_dir).context("Failed to create logs directory")?;
    let (_guard, _log_writer) = init_tui_logging(&log_dir, 10)?;  // 10MB max file size

    info!("Starting TUI with IPC connection to daemon...");

    // Connect to daemon (will spawn if needed)
    let daemon_addr = connect_to_daemon().await?;
    info!("Connected to daemon at {}", daemon_addr);

    // Create IPC daemon client
    let daemon_client: Arc<dyn DaemonClient> = Arc::new(
        IpcDaemonClient::new(daemon_addr.clone()).await?
    );

    // Get initial system status from daemon
    let initial_status = match daemon_client.get_system_status().await {
        Ok(status) => {
            info!("Got system status from daemon: stage={}", status.stage);
            status
        }
        Err(e) => {
            info!("Could not get status from daemon, using default: {}", e);
            SystemStatusSnapshot::default()
        }
    };

    // Get or create default session
    let session_id = match daemon_client.list_sessions().await {
        Ok(sessions) => {
            if let Some(session) = sessions.iter().find(|s| s.name == "default") {
                info!("Found existing default session: {}", session.id);
                Some(session.id.clone())
            } else {
                // Create a new default session
                info!("Creating new default session...");
                match daemon_client.create_session(Some("default".to_string()), ".".to_string()).await {
                    Ok(session_id) => {
                        info!("Created session: {}", session_id);
                        Some(session_id)
                    }
                    Err(e) => {
                        info!("Could not create session: {}", e);
                        Some("default".to_string())
                    }
                }
            }
        }
        Err(e) => {
            info!("Could not list sessions: {}", e);
            Some("default".to_string())
        }
    };

    // Run TUI with IPC client
    info!("Starting TUI with session: {:?}", session_id);
    run_tui(Some(initial_status), Some(daemon_client), session_id).await?;

    Ok(())
}
