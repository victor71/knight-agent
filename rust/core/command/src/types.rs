//! Command Types
//!
//! Core data types for the command system.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Command errors
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Command not initialized")]
    NotInitialized,
    #[error("Command not found: {0}")]
    NotFound(String),
    #[error("Command parsing failed: {0}")]
    ParseError(String),
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Argument error: {0}")]
    ArgError(String),
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Invalid command definition: {0}")]
    InvalidDefinition(String),
    #[error("Variable error: {0}")]
    VariableError(String),
}

/// Result type for command operations
pub type CommandResult<T> = Result<T, CommandError>;

/// Command type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CommandType {
    #[default]
    Simple,
    Workflow,
}

/// Command metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMetadata {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub file_path: String,
    #[serde(default)]
    pub command_type: CommandType,
}

impl CommandMetadata {
    pub fn new(name: &str, description: &str, file_path: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            version: None,
            author: None,
            file_path: file_path.to_string(),
            command_type: CommandType::Simple,
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    pub fn with_command_type(mut self, command_type: CommandType) -> Self {
        self.command_type = command_type;
        self
    }
}

/// Command argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArg {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub type_hint: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
}

impl CommandArg {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required: false,
            type_hint: None,
            default: None,
        }
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn with_type_hint(mut self, type_hint: &str) -> Self {
        self.type_hint = Some(type_hint.to_string());
        self
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }
}

/// Command usage info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandUsage {
    pub syntax: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub expected_behavior: Option<String>,
}

impl CommandUsage {
    pub fn new(syntax: &str) -> Self {
        Self {
            syntax: syntax.to_string(),
            examples: Vec::new(),
            expected_behavior: None,
        }
    }

    pub fn with_examples(mut self, examples: Vec<String>) -> Self {
        self.examples = examples;
        self
    }

    pub fn with_expected_behavior(mut self, behavior: &str) -> Self {
        self.expected_behavior = Some(behavior.to_string());
        self
    }
}

/// Workflow configuration for workflow-type commands
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub workflow_definition_path: Option<String>,
    #[serde(default)]
    pub dynamic_agent_creation: bool,
    #[serde(default)]
    pub parallel_execution: bool,
}

/// Command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub metadata: CommandMetadata,
    pub usage: CommandUsage,
    #[serde(default)]
    pub args: Vec<CommandArg>,
    #[serde(default)]
    pub workflow_config: Option<WorkflowConfig>,
}

impl CommandDefinition {
    pub fn new(name: &str, description: &str, syntax: &str, file_path: &str) -> Self {
        Self {
            metadata: CommandMetadata::new(name, description, file_path),
            usage: CommandUsage::new(syntax),
            args: Vec::new(),
            workflow_config: None,
        }
    }

    pub fn with_args(mut self, args: Vec<CommandArg>) -> Self {
        self.args = args;
        self
    }

    pub fn with_workflow_config(mut self, config: WorkflowConfig) -> Self {
        self.metadata.command_type = CommandType::Workflow;
        self.workflow_config = Some(config);
        self
    }
}

/// Parsed arguments
pub type ParsedArgs = serde_json::Map<String, serde_json::Value>;

/// Command execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionContext {
    pub command: CommandDefinition,
    pub parsed_args: ParsedArgs,
    #[serde(default)]
    pub user_input: String,
    #[serde(default)]
    pub session_id: Option<String>,
}

impl CommandExecutionContext {
    pub fn new(command: CommandDefinition, parsed_args: ParsedArgs) -> Self {
        Self {
            command,
            parsed_args,
            user_input: String::new(),
            session_id: None,
        }
    }

    pub fn with_user_input(mut self, input: &str) -> Self {
        self.user_input = input.to_string();
        self
    }

    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub success: bool,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub execution_time_ms: u64,
}

impl CommandExecutionResult {
    pub fn success(output: &str) -> Self {
        Self {
            success: true,
            output: output.to_string(),
            data: None,
            error: None,
            execution_time_ms: 0,
        }
    }

    pub fn success_with_data(output: &str, data: serde_json::Value) -> Self {
        Self {
            success: true,
            output: output.to_string(),
            data: Some(data),
            error: None,
            execution_time_ms: 0,
        }
    }

    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            output: String::new(),
            data: None,
            error: Some(error.to_string()),
            execution_time_ms: 0,
        }
    }

    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.execution_time_ms = time_ms;
        self
    }
}

/// Command info for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub command_type: CommandType,
    #[serde(default)]
    pub args: Vec<CommandArg>,
}

impl CommandInfo {
    pub fn from_definition(def: &CommandDefinition) -> Self {
        Self {
            name: def.metadata.name.clone(),
            description: def.metadata.description.clone(),
            command_type: def.metadata.command_type,
            args: def.args.clone(),
        }
    }
}

/// Command registry entry
#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub definition: CommandDefinition,
    pub enabled: bool,
}

impl CommandEntry {
    pub fn new(definition: CommandDefinition) -> Self {
        Self {
            definition,
            enabled: true,
        }
    }
}

/// Built-in function for variable resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFunction {
    Timestamp,
    Date,
    Default,
    Upper,
    Lower,
}

impl BuiltinFunction {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "timestamp" => Some(Self::Timestamp),
            "date" => Some(Self::Date),
            "default" => Some(Self::Default),
            "upper" => Some(Self::Upper),
            "lower" => Some(Self::Lower),
            _ => None,
        }
    }

    pub fn apply(&self, value: &str, arg: Option<&str>) -> String {
        match self {
            Self::Timestamp => chrono::Utc::now().timestamp().to_string(),
            Self::Date => {
                if let Some(format) = arg {
                    chrono::Utc::now().format(format).to_string()
                } else {
                    chrono::Utc::now().format("%Y-%m-%d").to_string()
                }
            }
            Self::Default => arg.unwrap_or(value).to_string(),
            Self::Upper => value.to_uppercase(),
            Self::Lower => value.to_lowercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_metadata_new() {
        let meta = CommandMetadata::new("test", "Test command", "test.md");
        assert_eq!(meta.name, "test");
        assert_eq!(meta.command_type, CommandType::Simple);
    }

    #[test]
    fn test_command_definition_new() {
        let cmd = CommandDefinition::new("review", "Code review", "/review [path]", "review.md");
        assert_eq!(cmd.metadata.name, "review");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_command_definition_with_args() {
        let args = vec![
            CommandArg::new("path", "File path").with_required(true),
            CommandArg::new("type", "Review type").with_default("quick"),
        ];
        let cmd = CommandDefinition::new("review", "Code review", "/review [path]", "review.md")
            .with_args(args);

        assert_eq!(cmd.args.len(), 2);
        assert!(cmd.args[0].required);
        assert_eq!(cmd.args[1].default.as_deref(), Some("quick"));
    }

    #[test]
    fn test_command_execution_result_success() {
        let result = CommandExecutionResult::success("Done");
        assert!(result.success);
        assert_eq!(result.output, "Done");
    }

    #[test]
    fn test_command_execution_result_failure() {
        let result = CommandExecutionResult::failure("Error occurred");
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Error occurred"));
    }

    #[test]
    fn test_builtin_function_timestamp() {
        let func = BuiltinFunction::Timestamp;
        let result = func.apply("", None);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_builtin_function_upper() {
        let func = BuiltinFunction::Upper;
        assert_eq!(func.apply("hello", None), "HELLO");
    }

    #[test]
    fn test_builtin_function_lower() {
        let func = BuiltinFunction::Lower;
        assert_eq!(func.apply("HELLO", None), "hello");
    }

    #[test]
    fn test_command_info_from_definition() {
        let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
        let info = CommandInfo::from_definition(&cmd);
        assert_eq!(info.name, "test");
    }
}
