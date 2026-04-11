//! Agent Runtime Types
//!
//! Core data types for the agent runtime system.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Agent runtime errors
#[derive(Error, Debug)]
pub enum AgentRuntimeError {
    #[error("Agent runtime not initialized")]
    NotInitialized,
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Agent already running: {0}")]
    AlreadyRunning(String),
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),
    #[error("LLM call failed: {0}")]
    LlmCallFailed(String),
    #[error("Context update failed: {0}")]
    ContextUpdateFailed(String),
    #[error("Operation cancelled: {0}")]
    OperationCancelled(String),
    #[error("Timeout: {0}")]
    Timeout(String),
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),
}

/// Result type for agent runtime operations
pub type RuntimeResult<T> = Result<T, AgentRuntimeError>;

/// Agent status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AgentStatus {
    #[default]
    Idle,
    Thinking,
    Acting,
    Paused,
    AwaitingUser,
    Error,
    Stopped,
}

/// Agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub status: AgentStatus,
    #[serde(default)]
    pub current_action: Option<String>,
    #[serde(default)]
    pub error: Option<ErrorInfo>,
    pub statistics: AgentStatistics,
    #[serde(default)]
    pub await_info: Option<AwaitInfo>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            status: AgentStatus::Idle,
            current_action: None,
            error: None,
            statistics: AgentStatistics::default(),
            await_info: None,
        }
    }
}

impl AgentState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_status(mut self, status: AgentStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_action(mut self, action: String) -> Self {
        self.current_action = Some(action);
        self
    }

    pub fn with_error(mut self, error: ErrorInfo) -> Self {
        self.status = AgentStatus::Error;
        self.error = Some(error);
        self
    }
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub retryable: bool,
}

impl ErrorInfo {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            retryable: false,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

/// Await info for user responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitInfo {
    pub await_id: String,
    pub query_type: String,
    pub message: String,
    pub created_at: String,
}

impl AwaitInfo {
    pub fn new(await_id: &str, query_type: &str, message: &str) -> Self {
        Self {
            await_id: await_id.to_string(),
            query_type: query_type.to_string(),
            message: message.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Agent statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStatistics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub tools_called: u64,
    pub llm_calls: u64,
    pub total_tokens: u64,
    pub errors: u64,
}

impl AgentStatistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_messages_sent(&mut self) {
        self.messages_sent += 1;
    }

    pub fn increment_messages_received(&mut self) {
        self.messages_received += 1;
    }

    pub fn increment_tools_called(&mut self) {
        self.tools_called += 1;
    }

    pub fn increment_llm_calls(&mut self) {
        self.llm_calls += 1;
    }

    pub fn add_tokens(&mut self, tokens: u64) {
        self.total_tokens += tokens;
    }

    pub fn increment_errors(&mut self) {
        self.errors += 1;
    }
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Content block type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockType {
    Text,
    Image,
    ToolUse,
    ToolResult,
}

/// Content block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: ContentBlockType,
    pub content: serde_json::Value,
}

/// Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: serde_json::Value,
    pub timestamp: String,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl Message {
    pub fn new(role: MessageRole, content: impl Into<serde_json::Value>) -> Self {
        Self {
            role,
            content: content.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: serde_json::Map::new(),
        }
    }

    pub fn user(content: impl Into<serde_json::Value>) -> Self {
        Self::new(MessageRole::User, content)
    }

    pub fn assistant(content: impl Into<serde_json::Value>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    pub fn system(content: impl Into<serde_json::Value>) -> Self {
        Self::new(MessageRole::System, content)
    }

    pub fn tool(content: impl Into<serde_json::Value>) -> Self {
        Self::new(MessageRole::Tool, content)
    }
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ToolResult {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            duration_ms: 0,
        }
    }

    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
            duration_ms: 0,
        }
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub key: String,
    pub value: serde_json::Value,
    pub timestamp: String,
}

impl MemoryItem {
    pub fn new(key: &str, value: serde_json::Value) -> Self {
        Self {
            key: key.to_string(),
            value,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Agent context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub variables: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub memory: Vec<MemoryItem>,
}

impl AgentContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn set_variable(&mut self, key: &str, value: serde_json::Value) {
        self.variables.insert(key.to_string(), value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    pub fn add_memory(&mut self, item: MemoryItem) {
        self.memory.push(item);
    }
}

/// Agent instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub definition_id: String,
    pub session_id: String,
    #[serde(default)]
    pub variant: Option<String>,
    pub state: AgentState,
    pub context: AgentContext,
}

impl Agent {
    pub fn new(id: String, definition_id: String, session_id: String) -> Self {
        Self {
            id,
            definition_id,
            session_id,
            variant: None,
            state: AgentState::new(),
            context: AgentContext::new(),
        }
    }

    pub fn with_variant(mut self, variant: String) -> Self {
        self.variant = Some(variant);
        self
    }

    pub fn is_idle(&self) -> bool {
        self.state.status == AgentStatus::Idle
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self.state.status,
            AgentStatus::Thinking | AgentStatus::Acting | AgentStatus::AwaitingUser
        )
    }

    pub fn is_stopped(&self) -> bool {
        self.state.status == AgentStatus::Stopped
    }
}

/// User response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub await_id: String,
    pub response: serde_json::Value,
    pub approved: bool,
}

impl UserResponse {
    pub fn new(await_id: &str, response: serde_json::Value, approved: bool) -> Self {
        Self {
            await_id: await_id.to_string(),
            response,
            approved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_new() {
        let state = AgentState::new();
        assert_eq!(state.status, AgentStatus::Idle);
        assert!(state.current_action.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_agent_state_with_status() {
        let state = AgentState::new().with_status(AgentStatus::Thinking);
        assert_eq!(state.status, AgentStatus::Thinking);
    }

    #[test]
    fn test_agent_statistics() {
        let mut stats = AgentStatistics::new();
        stats.increment_messages_sent();
        stats.increment_tools_called();
        stats.add_tokens(100);

        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.tools_called, 1);
        assert_eq!(stats.total_tokens, 100);
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
    }

    #[test]
    fn test_agent_context() {
        let mut ctx = AgentContext::new();
        ctx.set_variable("name", serde_json::json!("test"));
        assert_eq!(ctx.get_variable("name"), Some(&serde_json::json!("test")));
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success(serde_json::json!({"key": "value"}));
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult::failure("error message");
        assert!(!result.success);
        assert_eq!(result.error, Some("error message".to_string()));
    }
}
