//! In-process mode - Single process mode for development/testing
//!
//! This mode runs all components in a single process, similar to the original
//! Knight Agent architecture. It's useful for development and testing.

use anyhow::{Context, Result};
use async_trait::async_trait;
use bootstrap::KnightAgentSystem;
use cli::{Cli, CliImpl};
use configuration::ConfigLoader;
use session_manager::AgentRuntimeProxy;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;
use tui::{DaemonClient, DirectDaemonClient, SystemStatusSnapshot};
use tui::event::SystemHealth;

use crate::{get_home_dir, ensure_dir, AGENT_SUBDIRS, CONFIG_DIR};

/// Adapter to connect agent-runtime with session-manager
pub(crate) struct AgentRuntimeAdapter {
    inner: Arc<dyn agent_runtime::AgentHandle>,
}

impl AgentRuntimeAdapter {
    pub(crate) fn new(inner: Arc<dyn agent_runtime::AgentHandle>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl AgentRuntimeProxy for AgentRuntimeAdapter {
    async fn get_or_create_session_agent(&self, session_id: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.get_or_create_session_agent(session_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn send_message(&self, agent_id: &str, content: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use agent_runtime::Message;
        let message = Message::user(content);
        let response = self.inner.send_message(agent_id, message, false).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        // Extract content from response
        Ok(response.content.to_string())
    }
}

/// System state shared across the application
pub(crate) struct AppState {
    pub(crate) system: KnightAgentSystem,
    pub(crate) cli: Arc<CliImpl>,
}

impl AppState {
    pub(crate) async fn new() -> Result<Self> {
        let config = bootstrap::BootstrapConfig::default();
        let system = KnightAgentSystem::with_config(config);
        let cli = CliImpl::new()
            .context("Failed to create CLI")?;

        Ok(Self { system, cli: Arc::new(cli) })
    }
}

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
    pub(crate) fn set_session(&self, session_id: String) -> Result<()> {
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
    pub(crate) fn get_current_log_path(&self) -> Option<PathBuf> {
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
pub(crate) fn check_system_config() -> Result<PathBuf> {
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

    // Create subdirectories: sessions, logs, skills, commands
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

/// Initialize logging with session-based rotating log files
pub(crate) fn init_logging(log_dir: &Path, config: &configuration::LoggingConfig) -> Result<(WorkerGuard, Arc<Mutex<SessionLogWriter>>)> {
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

/// Create an LLM provider from configuration
async fn create_llm_provider(
    _config_loader: &Arc<ConfigLoader>,
) -> Result<Option<llm_provider::GenericLLMProvider>> {
    // Try to load LLM config from knight.json
    // For now, return None to indicate no provider is configured
    // In production, this would load from config and create a real provider

    info!("LLM provider configuration not yet implemented, using placeholder");
    Ok(None)
}

/// Run the in-process mode
pub(crate) async fn run_in_process() -> Result<()> {
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

    info!("Starting Knight Agent (in-process mode)...");
    info!("Log level: {}", logging_config.level);
    info!("Max log file size: {} MB", logging_config.max_file_size_mb);

    // Initialize system
    let state = AppState::new().await?;

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

    // Create Router
    info!("Creating Router...");
    let router = Arc::new(router::RouterImpl::new());
    router.initialize().await?;
    info!("Router initialized");

    // Create Agent Runtime
    info!("Creating Agent Runtime...");
    let mut agent_runtime_impl = agent_runtime::AgentRuntimeImpl::new();
    agent_runtime_impl.initialize().await?;

    // Create LLM Provider from config (using default/minimal config for now)
    // In production, this would be configured via knight.json
    info!("Creating LLM Provider...");
    let llm_provider = create_llm_provider(&config_loader).await?;
    if let Some(provider) = llm_provider {
        let provider = Arc::new(provider);
        agent_runtime_impl.set_llm_provider(provider.clone());
        info!("LLM Provider configured");
    }

    // Wrap in Arc for sharing
    let agent_runtime: Arc<dyn agent_runtime::AgentHandle> = Arc::new(agent_runtime_impl);
    info!("Agent Runtime initialized");

    // Create Session Manager and connect with Agent Runtime
    info!("Creating Session Manager...");
    let session_manager = Arc::new(session_manager::SessionManagerImpl::new());
    session_manager.initialize().await?;

    // Create adapter and set agent runtime for session manager
    let adapter = AgentRuntimeAdapter::new(agent_runtime.clone());
    session_manager.set_agent_runtime(Arc::new(adapter)).await;
    info!("Session Manager initialized and connected to Agent Runtime");

    // Run CLI TUI (check for --no-tui flag)
    let use_tui = !std::env::args().any(|arg| arg == "--no-tui");

    if use_tui {
        info!("Starting TUI...");

        // Create initial system status snapshot for TUI
        let stage_name = bootstrap::BootstrapStage::from_u8(status.stage)
            .map(|s| s.name().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let initial_status = SystemStatusSnapshot {
            system_status: if status.ready { SystemHealth::Healthy } else { SystemHealth::Degraded },
            stage: stage_name,
            module_count: status.module_count,
            initialized_count: status.initialized_count,
            uptime: Duration::ZERO,
            cpu_usage: 0.0,
            memory_usage: 0,
        };

        // Create daemon client with router and session manager
        let daemon_client: Arc<dyn DaemonClient> = Arc::new(
            DirectDaemonClient::new()
                .with_router(router.clone())
                .with_session_manager(session_manager.clone())
        );

        state.cli.run_tui(
            Some(initial_status),
            Some(daemon_client),
            Some("default".to_string()),
        ).await?;
    } else {
        // Run CLI REPL (fallback)
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

        state.cli.run_repl().await?;
    }

    // Shutdown
    info!("Shutting down...");
    state.system.stop(true, 5000).await?;

    println!("Knight Agent stopped. Goodbye!");
    Ok(())
}
