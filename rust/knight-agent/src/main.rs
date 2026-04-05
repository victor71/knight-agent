//! Knight Agent - Main Entry Point
//!
//! This binary initializes and runs the Knight Agent system.

use anyhow::Result;
use bootstrap::{BootstrapConfig, KnightAgentSystem};
use cli::{Cli, CliImpl};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// System state shared across the application
struct AppState {
    system: KnightAgentSystem,
    cli: Arc<CliImpl>,
}

impl AppState {
    async fn new() -> Result<Self> {
        let config = BootstrapConfig::default();
        let system = KnightAgentSystem::with_config(config);
        let cli = CliImpl::new()?;

        Ok(Self { system, cli: Arc::new(cli) })
    }
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

    // Run CLI REPL
    info!("Starting CLI REPL...");
    println!();
    println!("========================================");
    println!("  Knight Agent v{}", state.system.version().version);
    println!("========================================");
    println!();

    state.cli.repl().run().await?;

    // Shutdown
    info!("Shutting down...");
    state.system.stop(true, 5000).await?;

    println!("Knight Agent stopped. Goodbye!");
    Ok(())
}
