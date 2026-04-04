//! Tool System Types
//!
//! Core data types for the tool framework.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool definition with handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub category: String,
    pub parameters: JsonSchema,
    pub handler: ToolHandler,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub dangerous: bool,
    /// If true, this tool only reads data and can be executed in parallel
    #[serde(default)]
    pub is_read_only: bool,
}

/// JSON Schema for tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonSchema {
    #[serde(rename = "type", default)]
    pub schema_type: String,
    #[serde(default)]
    pub properties: HashMap<String, JsonSchemaProperty>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(rename = "additionalProperties", default)]
    pub additional_properties: bool,
}

/// JSON Schema property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaProperty {
    #[serde(rename = "type", default)]
    pub property_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enum_values: Option<Vec<String>>,
}

/// Tool handler type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HandlerType {
    Builtin,
    Command,
    Skill,
    #[serde(rename = "mcp")]
    Mcp,
    Wasm,
}

/// Tool handler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolHandler {
    #[serde(rename = "type")]
    pub handler_type: HandlerType,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub timeout_secs: u64,
}

/// Tool information (without handler)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub parameters: JsonSchema,
    pub dangerous: bool,
    /// If true, this tool only reads data and can be executed in parallel
    #[serde(default)]
    pub is_read_only: bool,
}

/// Tool context for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub session_id: String,
    pub agent_id: String,
    pub workspace: String,
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(rename = "error_code", default)]
    pub error_code: Option<String>,
    #[serde(rename = "duration_ms", default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolExecutionResult {
    /// Create a success result
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            error_code: None,
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create an error result
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
            error_code: Some(code.to_string()),
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Set duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub server_name: String,
}

/// Tool execution request
#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    pub name: String,
    pub args: serde_json::Value,
    pub context: ToolContext,
}

/// Argument validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: &str, message: &str) -> Self {
        Self {
            field: field.to_string(),
            message: message.to_string(),
        }
    }
}

/// Argument validation result
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }

    pub fn add_error(&mut self, field: &str, message: &str) {
        self.valid = false;
        self.errors.push(ValidationError::new(field, message));
    }
}
