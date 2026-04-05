//! Knight Agent - Main Entry Point
//!
//! This binary initializes and runs the Knight Agent system.

use anyhow::{Context, Result};
use bootstrap::{BootstrapConfig, KnightAgentSystem};
use cli::{Cli, CliImpl};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Default configuration directory name
const CONFIG_DIR: &str = ".knight-agent";
/// Default configuration file name
const CONFIG_FILE: &str = "config.toml";

/// System state shared across the application
struct AppState {
    system: KnightAgentSystem,
    cli: Arc<CliImpl>,
}

impl AppState {
    async fn new(_config_dir: &Path) -> Result<Self> {
        let config = BootstrapConfig::default();
        let system = KnightAgentSystem::with_config(config);
        let cli = CliImpl::new()
            .context("Failed to create CLI")?;

        Ok(Self { system, cli: Arc::new(cli) })
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

/// Check if the system is properly configured
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

    // Create subdirectories: sessions, logs, skills, commands
    for subdir in AGENT_SUBDIRS {
        let dir_path = config_dir.join(subdir);
        ensure_dir(&dir_path, subdir)?;
    }

    // Check config file
    let config_file = config_dir.join(CONFIG_FILE);
    if config_file.exists() {
        info!("Config file exists: {}", config_file.display());
        // Validate config file is readable
        std::fs::read_to_string(&config_file)
            .context("Failed to read config file")?;
        info!("Config file is readable");
    } else {
        warn!("Config file not found: {} (using defaults)", config_file.display());
    }

    // Check and create workspace directory inside .knight-agent
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
    println!("    - sessions/");
    println!("    - logs/");
    println!("    - skills/");
    println!("    - commands/");
    println!("    - workspace/");
    println!("  Config file: config.toml");
    println!();
}

/// Display help information
#[allow(dead_code)]
fn display_help() {
    println!();
    println!("Knight Agent CLI Commands:");
    println!("  /help, /h         - Show this help");
    println!("  /status           - Show system status");
    println!("  /sessions         - List sessions");
    println!("  /sessions new     - Create new session");
    println!("  /sessions switch   - Switch session");
    println!("  /agents           - List agents");
    println!("  /quit, /exit      - Exit CLI");
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Run system configuration check first to get config directory
    let config_dir = match check_system_config() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("System configuration check failed: {}", e);
            // Continue anyway, using defaults
            PathBuf::from(".").join(CONFIG_DIR)
        }
    };

    // Set up log file path
    let log_dir = config_dir.join("logs");
    let log_file = log_dir.join("knight-agent.log");

    // Create a file-based subscriber for logging
    let file_appender = tracing_appender::rolling::daily(&log_dir, "knight-agent.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(file_writer)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Knight Agent...");

    // Initialize system
    let state = AppState::new(&config_dir).await?;

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
    println!("Logs are written to: {}", log_file.display());
    println!();

    state.cli.repl().run().await?;

    // Shutdown
    info!("Shutting down...");
    state.system.stop(true, 5000).await?;

    println!("Knight Agent stopped. Goodbye!");
    Ok(())
}
