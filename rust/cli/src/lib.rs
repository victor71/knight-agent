//! CLI (命令行接口)
//!
//! Design Reference: docs/03-module-design/cli/cli.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Daemon not running")]
    DaemonNotRunning,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Timeout")]
    Timeout,
    #[error("IPC error: {0}")]
    IpcError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonAction {
    Start,
    Stop,
    Status,
    Restart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplInput {
    SlashCommand { command: String, args: String },
    NaturalLanguage { text: String },
    Empty,
}

impl ReplInput {
    pub fn parse(line: &str) -> Self {
        let line = line.trim();
        if line.is_empty() {
            return ReplInput::Empty;
        }
        if line.starts_with('/') {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            let command = parts[0].trim_start_matches('/').to_string();
            let args = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
            return ReplInput::SlashCommand { command, args };
        }
        ReplInput::NaturalLanguage {
            text: line.to_string(),
        }
    }
}

pub trait Cli: Send + Sync {
    fn new() -> Result<Self, CliError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn run_repl(&self) -> Result<(), CliError>;
    async fn daemon_action(&self, action: DaemonAction) -> Result<(), CliError>;
    async fn health_check(&self) -> Result<(), CliError>;
}

pub struct CliImpl;

impl Cli for CliImpl {
    fn new() -> Result<Self, CliError> {
        Ok(CliImpl)
    }

    fn name(&self) -> &str {
        "cli"
    }

    fn is_initialized(&self) -> bool {
        false // TODO: implement
    }

    async fn run_repl(&self) -> Result<(), CliError> {
        Ok(()) // TODO: implement
    }

    async fn daemon_action(&self, _action: DaemonAction) -> Result<(), CliError> {
        Ok(()) // TODO: implement
    }

    async fn health_check(&self) -> Result<(), CliError> {
        Ok(()) // TODO: implement
    }
}
