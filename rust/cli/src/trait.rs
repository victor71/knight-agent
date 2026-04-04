//! CLI trait definition

use async_trait::async_trait;

use crate::error::CliResult;
use crate::types::DaemonAction;

/// CLI trait
#[async_trait::async_trait]
pub trait Cli: Send + Sync {
    /// Create a new CLI instance
    fn new() -> CliResult<Self>
    where
        Self: Sized;

    /// Get the name of this CLI
    fn name(&self) -> &str;

    /// Check if the CLI is initialized
    fn is_initialized(&self) -> bool;

    /// Initialize the CLI
    async fn initialize(&self) -> CliResult<()>;

    /// Run the REPL
    async fn run_repl(&self) -> CliResult<()>;

    /// Execute a daemon action
    async fn daemon_action(&self, action: DaemonAction) -> CliResult<()>;

    /// Perform health check
    async fn health_check(&self) -> CliResult<()>;

    /// Stop the CLI
    async fn stop(&self) -> CliResult<()>;
}
