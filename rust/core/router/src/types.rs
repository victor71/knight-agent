//! Router Types
//!
//! Core data types for the routing system.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Router errors
#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Router not initialized")]
    NotInitialized,
    #[error("Route not found: {0}")]
    RouteNotFound(String),
    #[error("Routing failed: {0}")]
    RoutingFailed(String),
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
}

/// Result type for router operations
pub type RouterResult<T> = Result<T, RouterError>;

/// Command handler type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandHandlerType {
    Builtin,
    Session,
    Agent,
    CommandModule,
    TaskManager,
}

/// Command handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHandler {
    pub handler_type: CommandHandlerType,
    pub name: String,
}

/// Built-in command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinCommand {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub handler: CommandHandler,
}

/// User-defined command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCommand {
    pub name: String,
    pub description: String,
    pub template: String,
    pub variables: Vec<CommandVariable>,
    pub handler: CommandHandler,
}

/// Command variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandVariable {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

/// Command info for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub command_type: CommandType,
    pub aliases: Vec<String>,
}

impl CommandInfo {
    pub fn builtin(name: &str, description: &str, aliases: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            command_type: CommandType::Builtin,
            aliases,
        }
    }

    pub fn user(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            command_type: CommandType::User,
            aliases: Vec::new(),
        }
    }

    pub fn workflow(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            command_type: CommandType::Workflow,
            aliases: Vec::new(),
        }
    }
}

/// Command type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    Builtin,
    User,
    Workflow,
}

/// Route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub handler: CommandHandler,
    pub middleware: Vec<String>,
}

/// Router response
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterResponse {
    pub success: bool,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub to_agent: bool,
}

impl RouterResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            error: None,
            to_agent: false,
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            error: None,
            to_agent: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: String::new(),
            data: None,
            error: Some(message.into()),
            to_agent: false,
        }
    }

    pub fn forwarded_to_agent(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            error: None,
            to_agent: true,
        }
    }
}

/// Parsed input
#[derive(Debug, Clone)]
pub struct ParsedInput {
    pub is_command: bool,
    pub command_name: Option<String>,
    pub args: Vec<String>,
    pub raw_input: String,
}

impl ParsedInput {
    pub fn new(raw_input: impl Into<String>) -> Self {
        let raw = raw_input.into();
        let trimmed = raw.trim();

        if trimmed.starts_with('/') {
            let parts: Vec<&str> = trimmed[1..].splitn(2, |c: char| c.is_whitespace()).collect();
            let command_name = parts.first().map(|s| s.to_lowercase());
            let args = if parts.len() > 1 {
                parts[1].split_whitespace().map(|s| s.to_string()).collect()
            } else {
                Vec::new()
            };

            Self {
                is_command: true,
                command_name,
                args,
                raw_input: raw,
            }
        } else {
            Self {
                is_command: false,
                command_name: None,
                args: Vec::new(),
                raw_input: raw,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.raw_input.trim().is_empty()
    }
}

/// Input handling request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleInputRequest {
    pub input: String,
    pub session_id: String,
}

/// Input handling result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleInputResult {
    pub response: RouterResponse,
    pub to_agent: bool,
}
