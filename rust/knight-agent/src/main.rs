//! Knight Agent - Main Entry Point
//!
//! This binary initializes and runs the Knight Agent system.

use anyhow::{Context, Result};
use bootstrap::{BootstrapConfig, KnightAgentSystem};
use cli::{Cli, CliImpl};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Default configuration directory name
const CONFIG_DIR: &str = ".knight-agent";
/// Default configuration file name
const CONFIG_FILE: &str = "config.toml";
/// Default workspace directory
const WORKSPACE_DIR: &str = ".knight-workspace";

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

/// Check if the system is properly configured
fn check_system_config() -> Result<()> {
    info!("Checking system configuration...");

    // Check current directory
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    info!("Working directory: {}", current_dir.display());

    // Check .knight-agent directory
    let config_dir = current_dir.join(CONFIG_DIR);
    if config_dir.exists() {
        info!("Config directory exists: {}", config_dir.display());
    } else {
        warn!("Config directory not found: {} (will be created on first run)", config_dir.display());
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

    // Check workspace directory
    let workspace_dir = current_dir.join(WORKSPACE_DIR);
    if workspace_dir.exists() {
        info!("Workspace directory exists: {}", workspace_dir.display());
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
    } else {
        info!("Workspace directory not found: {} (will be created on first run)", workspace_dir.display());
    }

    // Check dependencies
    check_dependencies()?;

    info!("System configuration check passed");
    Ok(())
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
fn display_banner(version: &str) {
    println!();
    println!("========================================");
    println!("  Knight Agent v{}", version);
    println!("========================================");
    println!();
    println!("Configuration:");
    println!("  Config dir: .knight-agent/");
    println!("  Config file: config.toml");
    println!("  Workspace: .knight-workspace/");
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
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Knight Agent...");

    // Run system configuration check
    if let Err(e) = check_system_config() {
        warn!("System configuration check failed: {}", e);
        // Continue anyway, using defaults
    }

    // Initialize system
    let state = AppState::new(Path::new(CONFIG_DIR)).await?;

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
    display_banner(&state.system.version().version);
    println!("Type /help for available commands, /quit to exit");
    println!();

    state.cli.repl().run().await?;

    // Shutdown
    info!("Shutting down...");
    state.system.stop(true, 5000).await?;

    println!("Knight Agent stopped. Goodbye!");
    Ok(())
}
