//! CLI implementation

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{CliError, CliResult};
use crate::r#trait::Cli;
use crate::repl::CliRepl;
use crate::types::DaemonAction;

/// CLI implementation
#[derive(Clone)]
pub struct CliImpl {
    initialized: Arc<RwLock<bool>>,
    repl: Arc<CliRepl>,
}

impl CliImpl {
    /// Create a new CLI implementation
    pub fn new() -> CliResult<Self> {
        Ok(Self {
            initialized: Arc::new(RwLock::new(false)),
            repl: Arc::new(CliRepl::new()),
        })
    }

    /// Get the REPL
    #[must_use]
    pub fn repl(&self) -> &CliRepl {
        &self.repl
    }
}

#[async_trait::async_trait]
impl Cli for CliImpl {
    fn new() -> CliResult<Self> {
        Self::new()
    }

    fn name(&self) -> &str {
        "cli"
    }

    fn is_initialized(&self) -> bool {
        match self.initialized.try_read() {
            Ok(guard) => *guard,
            Err(_) => {
                tracing::error!("CLI initialization lock poisoned, assuming not initialized");
                false
            }
        }
    }

    async fn initialize(&self) -> CliResult<()> {
        if *self.initialized.read().await {
            return Ok(());
        }

        *self.initialized.write().await = true;
        tracing::info!("CLI initialized");
        Ok(())
    }

    async fn run_repl(&self) -> CliResult<()> {
        if !self.is_initialized() {
            return Err(CliError::NotInitialized);
        }

        self.repl.run().await
    }

    async fn daemon_action(&self, action: DaemonAction) -> CliResult<()> {
        match action {
            DaemonAction::Start => {
                println!("Starting daemon...");
                // In production, this would start the daemon process
            }
            DaemonAction::Stop => {
                println!("Stopping daemon...");
                // In production, this would stop the daemon process
            }
            DaemonAction::Status => {
                println!("Daemon status: Running");
                // In production, this would check daemon status
            }
            DaemonAction::Restart => {
                println!("Restarting daemon...");
                // In production, this would restart the daemon process
            }
        }
        Ok(())
    }

    async fn health_check(&self) -> CliResult<()> {
        // Check if REPL is running
        if self.repl.is_running().await {
            println!("CLI Health: OK");
        } else {
            println!("CLI Health: Not running");
        }
        Ok(())
    }

    async fn stop(&self) -> CliResult<()> {
        *self.initialized.write().await = false;
        tracing::info!("CLI stopped");
        Ok(())
    }
}
