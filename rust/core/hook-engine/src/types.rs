//! Hook Engine Types
//!
//! Core data types for the hook system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Hook engine errors
#[derive(Error, Debug)]
pub enum HookError {
    #[error("Hook engine not initialized")]
    NotInitialized,
    #[error("Hook execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Hook not found: {0}")]
    NotFound(String),
    #[error("Hook already exists: {0}")]
    AlreadyExists(String),
    #[error("Hook disabled: {0}")]
    Disabled(String),
    #[error("Hook blocked: {0}")]
    Blocked(String),
}

/// Result type for hook operations
pub type HookResult<T> = Result<T, HookError>;

/// Hook phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum HookPhase {
    #[default]
    Before,
    After,
    Replace,
}


/// Hook handler type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookHandler {
    /// Command handler
    Command {
        executable: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    /// Skill handler
    Skill {
        skill_id: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    /// RPC handler
    Rpc {
        endpoint: String,
        #[serde(default = "default_rpc_method")]
        method: String,
        #[serde(default = "default_timeout")]
        timeout_secs: u64,
    },
    /// WASM handler
    Wasm {
        module: String,
        function: String,
    },
    /// Callback handler (internal use)
    Callback {
        #[serde(default)]
        handler: String,
    },
}

fn default_rpc_method() -> String {
    "POST".to_string()
}

fn default_timeout() -> u64 {
    30
}

/// Hook filter conditions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookFilter {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Hook control options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookControl {
    #[serde(default = "default_true")]
    pub can_block: bool,
    #[serde(default)]
    pub can_modify: bool,
    #[serde(default)]
    pub can_skip: bool,
    #[serde(default = "default_true")]
    pub continue_on_error: bool,
}

fn default_true() -> bool {
    true
}

impl Default for HookControl {
    fn default() -> Self {
        Self {
            can_block: false,
            can_modify: false,
            can_skip: false,
            continue_on_error: true,
        }
    }
}

/// Hook error handling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookErrorHandling {
    #[serde(default)]
    pub retry: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
    #[serde(default)]
    pub fallback: Option<String>,
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    1000
}

impl Default for HookErrorHandling {
    fn default() -> Self {
        Self {
            retry: false,
            max_retries: 3,
            retry_delay_ms: 1000,
            fallback: None,
        }
    }
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Event to listen for
    pub event: String,
    /// Execution phase
    pub phase: HookPhase,
    #[serde(default = "default_priority")]
    pub priority: u32,
    /// Filter conditions
    #[serde(default)]
    pub filter: HookFilter,
    /// Handler to execute
    pub handler: HookHandler,
    /// Control options
    #[serde(default)]
    pub control: HookControl,
    /// Error handling
    #[serde(default)]
    pub error_handling: HookErrorHandling,
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> u32 {
    100
}

impl HookDefinition {
    /// Create a new hook definition
    pub fn new(id: String, event: String, phase: HookPhase, handler: HookHandler) -> Self {
        Self {
            id,
            name: String::new(),
            description: String::new(),
            enabled: true,
            event,
            phase,
            priority: 100,
            filter: HookFilter::default(),
            handler,
            control: HookControl::default(),
            error_handling: HookErrorHandling::default(),
        }
    }
}

/// Hook context (runtime data passed during execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub event: String,
    pub phase: HookPhase,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub session: Option<SessionContext>,
    #[serde(default)]
    pub agent: Option<AgentContext>,
    #[serde(default)]
    pub request: Option<RequestContext>,
    #[serde(default)]
    pub response: Option<ResponseContext>,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

impl HookContext {
    /// Create a new basic context
    pub fn new(event: String, phase: HookPhase) -> Self {
        Self {
            event,
            phase,
            timestamp: chrono::Utc::now().to_rfc3339(),
            session: None,
            agent: None,
            request: None,
            response: None,
            data: HashMap::new(),
        }
    }

    /// Set session context
    pub fn with_session(mut self, session: SessionContext) -> Self {
        self.session = Some(session);
        self
    }

    /// Set agent context
    pub fn with_agent(mut self, agent: AgentContext) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Set request context
    pub fn with_request(mut self, request: RequestContext) -> Self {
        self.request = Some(request);
        self
    }

    /// Set response context
    pub fn with_response(mut self, response: ResponseContext) -> Self {
        self.response = Some(response);
        self
    }

    /// Add custom data
    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

/// Session context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub id: String,
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
}

/// Agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

/// Request context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Response context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContext {
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
}

/// Hook execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookExecutionResult {
    pub hook_id: String,
    pub success: bool,
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub block_reason: Option<String>,
    #[serde(default)]
    pub modified: bool,
    #[serde(default)]
    pub modified_data: Option<serde_json::Value>,
    #[serde(default)]
    pub skipped: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
}

impl HookExecutionResult {
    /// Create a successful result
    pub fn success(hook_id: String) -> Self {
        Self {
            hook_id,
            success: true,
            blocked: false,
            block_reason: None,
            modified: false,
            modified_data: None,
            skipped: false,
            error: None,
            duration_ms: 0,
        }
    }

    /// Create a blocked result
    pub fn blocked(hook_id: String, reason: String) -> Self {
        Self {
            hook_id,
            success: false,
            blocked: true,
            block_reason: Some(reason),
            modified: false,
            modified_data: None,
            skipped: false,
            error: None,
            duration_ms: 0,
        }
    }
}

/// Combined trigger result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggerResult {
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub block_reason: Option<String>,
    #[serde(default)]
    pub modified: bool,
    #[serde(default)]
    pub modified_data: Option<serde_json::Value>,
    #[serde(default)]
    pub skipped: bool,
    #[serde(default = "default_hooks_executed")]
    pub hooks_executed: u32,
    #[serde(default)]
    pub hooks_failed: u32,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub results: Vec<HookExecutionResult>,
}

fn default_hooks_executed() -> u32 {
    0
}

/// Hook information (for queries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInfo {
    pub id: String,
    pub name: String,
    pub event: String,
    pub phase: HookPhase,
    pub priority: u32,
    pub enabled: bool,
    #[serde(default)]
    pub execution_count: u64,
    #[serde(default)]
    pub last_executed: Option<String>,
}

impl From<&HookDefinition> for HookInfo {
    fn from(hook: &HookDefinition) -> Self {
        Self {
            id: hook.id.clone(),
            name: hook.name.clone(),
            event: hook.event.clone(),
            phase: hook.phase,
            priority: hook.priority,
            enabled: hook.enabled,
            execution_count: 0,
            last_executed: None,
        }
    }
}

/// Event point definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPoint {
    pub name: String,
    pub category: String,
    pub description: String,
    pub phases: Vec<HookPhase>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_definition_creation() {
        let handler = HookHandler::Skill {
            skill_id: "test_skill".to_string(),
            args: HashMap::new(),
        };
        let hook = HookDefinition::new("h1".to_string(), "tool_call".to_string(), HookPhase::Before, handler);

        assert_eq!(hook.id, "h1");
        assert_eq!(hook.event, "tool_call");
        assert_eq!(hook.phase, HookPhase::Before);
        assert!(hook.enabled);
    }

    #[test]
    fn test_hook_context_creation() {
        let ctx = HookContext::new("agent_execute".to_string(), HookPhase::Before)
            .with_session(SessionContext {
                id: "s1".to_string(),
                workspace: None,
                variables: HashMap::new(),
            })
            .with_data("key", serde_json::json!("value"));

        assert_eq!(ctx.event, "agent_execute");
        assert!(ctx.session.is_some());
        assert!(ctx.data.contains_key("key"));
    }

    #[test]
    fn test_hook_execution_result() {
        let result = HookExecutionResult::success("h1".to_string());
        assert!(result.success);
        assert!(!result.blocked);
    }
}
