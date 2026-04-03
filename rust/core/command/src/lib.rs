//! Command Registry
//!
//! Design Reference: docs/03-module-design/core/command.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub handler: String,
}

#[async_trait]
pub trait CommandRegistry: Send + Sync {
    fn new() -> Result<Self, CommandError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn register_command(&self, command: Command) -> Result<(), CommandError>;
    async fn execute_command(&self, name: &str) -> Result<serde_json::Value, CommandError>;
    async fn list_commands(&self) -> Result<Vec<Command>, CommandError>;
}

pub struct CommandRegistryImpl;

impl CommandRegistry for CommandRegistryImpl {
    fn new() -> Result<Self, CommandError> {
        Ok(CommandRegistryImpl)
    }

    fn name(&self) -> &str {
        "command-registry"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn register_command(&self, _command: Command) -> Result<(), CommandError> {
        Ok(())
    }

    async fn execute_command(&self, name: &str) -> Result<serde_json::Value, CommandError> {
        Err(CommandError::NotFound(name.to_string()))
    }

    async fn list_commands(&self) -> Result<Vec<Command>, CommandError> {
        Ok(vec![])
    }
}
