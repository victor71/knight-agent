//! Knight Agent - Main Entry Point
//!
//! This binary initializes and runs the Knight Agent system.

use anyhow::{Context, Result};
use bootstrap::{BootstrapConfig, KnightAgentSystem};
use cli::{Cli, CliImpl};
use knight_config::{ConfigLoader, LoggingConfig};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{info, warn, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;

/// Default configuration directory name
const CONFIG_DIR: &str = ".knight-agent";

/// System state shared across the application
struct AppState {
    system: KnightAgentSystem,
    cli: Arc<CliImpl>,
    config_loader: Arc<ConfigLoader>,
}

impl AppState {
    async fn new(config_loader: Arc<ConfigLoader>) -> Result<Self> {
        let config = BootstrapConfig::default();
        let system = KnightAgentSystem::with_config(config);
        let cli = CliImpl::new()
            .context("Failed to create CLI")?;

        Ok(Self { system, cli: Arc::new(cli), config_loader })
    }
}

/// Subdirectories to create under .knight-agent
const AGENT_SUBDIRS: &[&str] = &["sessions", "logs", "skills", "commands"];

/// Get the user's home directory for config storage
fn get_home_dir() -> Result<PathBuf> {
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
fn ensure_dir(path: &Path, name: &str) -> Result<bool> {
    if path.exists() {
        if path.is_dir() {
            info!("{} directory exists: {}", name, path.display());
            Ok(true)
        } else {
            warn!("{} path exists but is not a directory: {}", name, path.display());
            Ok(false)
        }
    } else {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create {} directory: {}", name, path.display()))?;
        info!("Created {} directory: {}", name, path.display());
        Ok(true)
    }
}

/// Session-based rotating log writer state
struct SessionLogWriter {
    log_dir: PathBuf,
    current_session_id: Mutex<Option<String>>,
    current_file: Mutex<Option<PathBuf>>,
    current_size: Mutex<u64>,
    file_index: Mutex<u32>,
    max_file_size: u64,
}

impl SessionLogWriter {
    fn new(log_dir: PathBuf, max_file_size: u64) -> Self {
        Self {
            log_dir,
            current_session_id: Mutex::new(None),
            current_file: Mutex::new(None),
            current_size: Mutex::new(0),
            file_index: Mutex::new(0),
            max_file_size,
        }
    }

    /// Generate a unique log file path for the session
    fn generate_log_path(&self, session_id: &str, index: u32) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = if index == 0 {
            format!("{}_{}.log", session_id, timestamp)
        } else {
            format!("{}_{}_{}.log", session_id, timestamp, index)
        };
        self.log_dir.join(filename)
    }

    /// Set the current session and create a new log file if needed
    fn set_session(&self, session_id: String) -> Result<()> {
        let mut current_session = self.current_session_id.lock().unwrap();

        // If same session, do nothing
        if current_session.as_ref() == Some(&session_id) {
            return Ok(());
        }

        // Update session
        *current_session = Some(session_id.clone());
        *self.file_index.lock().unwrap() = 0;
        *self.current_size.lock().unwrap() = 0;

        // Create new log file
        let log_path = self.generate_log_path(&session_id, 0);
        std::fs::write(&log_path, "").context("Failed to create log file")?;
        *self.current_file.lock().unwrap() = Some(log_path);

        info!("Created new log file for session: {}", session_id);
        Ok(())
    }

    /// Get the current log file path for display
    fn get_current_log_path(&self) -> Option<PathBuf> {
        self.current_file.lock().unwrap().clone()
    }

    /// Check if rotation is needed and rotate if file is too large
    fn check_rotation(&self) -> Result<()> {
        let current_session = self.current_session_id.lock().unwrap();
        let session_id = match current_session.as_ref() {
            Some(id) => id,
            None => return Ok(()), // No session set yet
        };

        let current_file = self.current_file.lock().unwrap();
        let file_path = match current_file.as_ref() {
            Some(p) => p,
            None => return Ok(()),
        };

        // Check file size
        let metadata = std::fs::metadata(file_path)?;
        let size = metadata.len();
        *self.current_size.lock().unwrap() = size;

        if size >= self.max_file_size {
            drop(current_file); // Release lock before creating new file
            let mut index = *self.file_index.lock().unwrap() + 1;
            *self.file_index.lock().unwrap() = index;

            // Rotate to a new file with index
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let mut new_path;

            loop {
                new_path = self.log_dir.join(format!("{}_{}_{}.log", session_id, timestamp, index));
                if !new_path.exists() {
                    break;
                }
                index += 1;
            }

            std::fs::write(&new_path, "").context("Failed to create rotated log file")?;
            *self.current_file.lock().unwrap() = Some(new_path.clone());
            *self.current_size.lock().unwrap() = 0;

            info!("Rotated log file to: {}", new_path.display());
        }

        Ok(())
    }

    /// Write data to the current log file
    fn write_data(&self, buf: &[u8]) -> std::io::Result<usize> {
        // Check if rotation is needed
        if let Err(e) = self.check_rotation() {
            eprintln!("Error checking log rotation: {}", e);
        }

        let current_file = self.current_file.lock().unwrap();
        let file_path = match current_file.as_ref() {
            Some(p) => p,
            None => return Ok(0), // No file open
        };

        // Open file in append mode
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        let result = IoWrite::write(&mut file, buf);

        // Update size if write succeeded
        if result.is_ok() {
            let mut size = self.current_size.lock().unwrap();
            *size += buf.len() as u64;
        }

        result
    }

    /// Flush the current log file
    fn flush_data(&self) -> std::io::Result<()> {
        let current_file = self.current_file.lock().unwrap();
        if let Some(file_path) = current_file.as_ref() {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(file_path)?;
            IoWrite::flush(&mut file)
        } else {
            Ok(())
        }
    }
}

/// Newtype wrapper to implement Write for Arc<Mutex<SessionLogWriter>>
struct LogWriter(Arc<Mutex<SessionLogWriter>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write_data(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush_data()
    }
}

/// Check if the system is properly configured and initialize config loader
fn check_system_config() -> Result<PathBuf> {
    info!("Checking system configuration...");

    // Use user's home directory for config storage
    let home_dir = get_home_dir()?;
    info!("User home directory: {}", home_dir.display());

    // Check and create .knight-agent directory with subdirectories
    let config_dir = home_dir.join(CONFIG_DIR);
    if !config_dir.exists() {
        info!("Creating .knight-agent directory: {}", config_dir.display());
        std::fs::create_dir_all(&config_dir)
            .context("Failed to create .knight-agent directory")?;
    }
    info!("Config directory ready: {}", config_dir.display());

    // Create subdirectories: sessions, logs, skills, commands, workspace
    for subdir in AGENT_SUBDIRS {
        let dir_path = config_dir.join(subdir);
        ensure_dir(&dir_path, subdir)?;
    }

    // Create workspace directory
    let workspace_dir = config_dir.join("workspace");
    if ensure_dir(&workspace_dir, "workspace")? {
        // Check workspace is writable
        let test_file = workspace_dir.join(".write_test");
        match std::fs::write(&test_file, "test") {
            Ok(_) => {
                std::fs::remove_file(&test_file)
                    .context("Failed to remove test file")?;
                info!("Workspace directory is writable");
            }
            Err(e) => {
                warn!("Workspace directory is not writable: {}", e);
            }
        }
    }

    // Check dependencies
    check_dependencies()?;

    info!("System configuration check passed");
    Ok(config_dir)
}

/// Check if required dependencies are available
fn check_dependencies() -> Result<()> {
    info!("Checking dependencies...");

    // On Windows, check for common tools using Command
    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("git").arg("--version").output() {
            Ok(output) if output.status.success() => {
                info!("Git is available");
            }
            _ => {
                warn!("Git not found in PATH (optional)");
            }
        }
    }

    info!("Dependencies check complete");
    Ok(())
}

/// Display startup banner
fn display_banner(version: &str, config_dir: &Path) {
    println!();
    println!("========================================");
    println!("  Knight Agent v{}", version);
    println!("========================================");
    println!();
    println!("Configuration:");
    println!("  Config dir: {}", config_dir.display());
    println!("    - knight.json        (LLM providers)");
    println!("    - config/");
    println!("      - agent.yaml       (agent modules)");
    println!("      - core.yaml        (core modules)");
    println!("      - services.yaml    (services)");
    println!("      - tools.yaml       (tool system)");
    println!("      - infrastructure.yaml");
    println!("      - storage.yaml");
    println!("      - security.yaml");
    println!("      - logging.yaml");
    println!("      - monitoring.yaml");
    println!("      - compressor.yaml");
    println!("    - sessions/");
    println!("    - logs/");
    println!("    - skills/");
    println!("    - commands/");
    println!("    - workspace/");
    println!();
}

/// Display help information
#[allow(dead_code)]
fn display_help() {
    println!();
    println!("Knight Agent CLI Commands:");
    println!("  /help, /h         - Show this help");
    println!("  /status           - Show system status");
    println!("  /config           - Show configuration");
    println!("  /sessions         - List sessions");
    println!("  /sessions new     - Create new session");
    println!("  /sessions switch   - Switch session");
    println!("  /agents           - List agents");
    println!("  /quit, /exit      - Exit CLI");
    println!();
}

/// Initialize logging with session-based rotating log files
fn init_logging(log_dir: &Path, config: &LoggingConfig) -> Result<(WorkerGuard, Arc<Mutex<SessionLogWriter>>)> {
    let max_file_size = config.max_file_size_mb * 1024 * 1024;

    // Create the session log writer with a default session
    let log_writer = Arc::new(Mutex::new(SessionLogWriter::new(log_dir.to_path_buf(), max_file_size)));

    // Set the default session
    log_writer.lock().unwrap().set_session("default".to_string())?;

    // Create a non-blocking writer from our session log writer wrapper
    let (file_writer, guard) = tracing_appender::non_blocking(LogWriter(log_writer.clone()));

    // Parse log level
    let log_level = match config.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // Build the subscriber - disable ANSI escape sequences for file logging
    let subscriber = tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)  // Disable ANSI color codes in log files
        .with_writer(file_writer)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok((guard, log_writer))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Run system configuration check first to get config directory
    let config_dir = match check_system_config() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("System configuration check failed: {}", e);
            // Continue anyway, using defaults
            let home = get_home_dir().unwrap_or_else(|_| PathBuf::from("."));
            home.join(CONFIG_DIR)
        }
    };

    // Initialize config loader (will create default configs if needed)
    let config_loader = Arc::new(ConfigLoader::new(config_dir.clone()).await
        .context("Failed to initialize config loader")?);

    info!("Config loader initialized: {}", config_dir.display());

    // Get logging configuration
    let logging_config = config_loader.get_logging_config();

    // Set up log directory
    let log_dir = config_dir.join("logs");

    // Initialize logging with session-based rotating logs
    let (_guard, log_writer) = init_logging(&log_dir, &logging_config)?;

    info!("Starting Knight Agent...");
    info!("Log level: {}", logging_config.level);
    info!("Max log file size: {} MB", logging_config.max_file_size_mb);

    // Initialize system
    let state = AppState::new(config_loader.clone()).await?;

    // Bootstrap the system through all 8 stages
    info!("Initializing system (8-stage bootstrap)...");
    state.system.bootstrap().await?;
    info!("System bootstrap complete!");

    // Print system status
    let status = state.system.status().await;
    info!(
        "Status: stage={}, initialized={}, ready={}, modules={}/{}",
        status.stage,
        status.initialized,
        status.ready,
        status.initialized_count,
        status.module_count
    );

    // Initialize CLI
    info!("Initializing CLI...");
    state.cli.initialize().await?;
    info!("CLI initialized");

    // Run CLI REPL
    display_banner(&state.system.version().version, &config_dir);
    println!("Type /help for available commands, /quit to exit");
    println!();

    // Display current log file location
    if let Some(log_path) = log_writer.lock().unwrap().get_current_log_path() {
        println!("Logs are written to: {}", log_path.display());
        println!("Log rotation: {} MB per file, max {} files",
            logging_config.max_file_size_mb,
            logging_config.max_files);
        println!();
    }

    // Start config change listener
    let config_loader_clone = config_loader.clone();
    tokio::spawn(async move {
        let mut rx = config_loader_clone.subscribe();
        while let Ok(change) = rx.recv().await {
            match change {
                knight_config::ConfigChangeEvent::MainConfigChanged(_) => {
                    info!("Main configuration reloaded");
                }
                knight_config::ConfigChangeEvent::SystemConfigChanged { name, .. } => {
                    info!("System configuration '{}' reloaded", name);
                }
            }
        }
    });

    state.cli.repl().run().await?;

    // Shutdown
    info!("Shutting down...");
    state.system.stop(true, 5000).await?;

    println!("Knight Agent stopped. Goodbye!");
    Ok(())
}
