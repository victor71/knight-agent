//! REPL implementation for CLI

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::signal::ctrl_c;

use crate::error::{CliError, CliResult};
use crate::types::{ReplCommand, ReplInput};

/// REPL state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplState {
    Running,
    Exiting,
}

/// CLI REPL implementation
#[derive(Clone)]
pub struct CliRepl {
    running: Arc<RwLock<bool>>,
    state: Arc<RwLock<ReplState>>,
    prompt: Arc<RwLock<String>>,
}

impl CliRepl {
    /// Create a new REPL
    pub fn new() -> Self {
        Self {
            running: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(ReplState::Running)),
            prompt: Arc::new(RwLock::new("knight>".to_string())),
        }
    }

    /// Check if running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get state
    pub async fn state(&self) -> ReplState {
        *self.state.read().await
    }

    /// Process a single input
    pub async fn process_input(&self, line: &str) -> CliResult<ReplCommand> {
        let input = ReplInput::parse(line);

        match input {
            ReplInput::Empty => Ok(ReplCommand::Status),
            ReplInput::SlashCommand { command, args } => {
                Ok(ReplCommand::parse(&command, &args))
            }
            ReplInput::NaturalLanguage { text } => {
                tracing::info!("Processing natural language input: {}", text);
                // In production, this would route to the orchestrator
                Ok(ReplCommand::Status)
            }
        }
    }

    /// Execute a command
    pub async fn execute_command(&self, command: ReplCommand) -> CliResult<()> {
        match command {
            ReplCommand::Help => {
                println!("Knight Agent CLI - Available commands:");
                println!("  /help, /h         - Show this help");
                println!("  /sessions         - List sessions");
                println!("  /sessions new      - Create new session");
                println!("  /sessions switch   - Switch session");
                println!("  /agents           - List agents");
                println!("  /status           - Show system status");
                println!("  /quit, /exit      - Exit CLI");
            }
            ReplCommand::SessionList => {
                println!("Sessions: (none)");
            }
            ReplCommand::SessionCreate { name } => {
                println!("Created session: {:?}", name);
            }
            ReplCommand::SessionSwitch { id } => {
                println!("Switched to session: {}", id);
            }
            ReplCommand::SessionDestroy { id } => {
                println!("Destroyed session: {}", id);
            }
            ReplCommand::AgentList => {
                println!("Agents: (none)");
            }
            ReplCommand::AgentSpawn { variant } => {
                println!("Spawned agent: {}", variant);
            }
            ReplCommand::Status => {
                println!("System Status: Running");
            }
            ReplCommand::Exit => {
                *self.state.write().await = ReplState::Exiting;
                *self.running.write().await = false;
            }
            ReplCommand::Quit => {
                *self.state.write().await = ReplState::Exiting;
                *self.running.write().await = false;
            }
            ReplCommand::Unknown { command } => {
                println!("Unknown command: {}. Type /help for available commands.", command);
            }
        }
        Ok(())
    }

    /// Run the REPL loop
    pub async fn run(&self) -> CliResult<()> {
        *self.running.write().await = true;

        println!("Knight Agent CLI");
        println!("Type /help for available commands, /quit to exit");

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        while self.is_running().await {
            // Print prompt
            print!("{}", *self.prompt.read().await);
            use std::io::Write;
            std::io::stdout().flush()?;

            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                // EOF
                break;
            }

            let command = self.process_input(&line).await?;
            self.execute_command(command).await?;

            if self.state().await == ReplState::Exiting {
                break;
            }
        }

        println!("Goodbye!");
        Ok(())
    }
}

impl Default for CliRepl {
    fn default() -> Self {
        Self::new()
    }
}
