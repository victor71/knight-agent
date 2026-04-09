//! Session process - Dedicated process for a single agent session
//!
//! A session process is spawned by the daemon to handle a single session's
//! agent interactions. It connects to the daemon via IPC.

use anyhow::{Context, Result};
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
