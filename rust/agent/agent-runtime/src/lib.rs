//! Agent Runtime
//!
//! Design Reference: docs/03-module-design/agent/agent-runtime.md
//!
//! This module provides the core agent runtime system, including:
//! - Agent lifecycle management (create, start, stop, pause, resume)
//! - Message handling and context management
//! - State machine for agent status transitions
//! - Tool call proxy interface

pub mod types;
pub mod runtime;

pub use types::{
    AgentRuntimeError, AgentStatus, AgentState, AgentStatistics, Agent, AgentContext,
    ErrorInfo, AwaitInfo, Message, MessageRole, ContentBlock, ContentBlockType,
    ToolResult, MemoryItem, UserResponse, RuntimeResult,
};

pub use runtime::AgentRuntimeImpl;

/// Configuration for the agent runtime
pub use runtime::RuntimeConfig;

/// AgentHandle trait for external consumers (Router, CLI, TUI)
#[async_trait::async_trait]
pub trait AgentHandle: Send + Sync {
    /// Send a message to an agent
    async fn send_message(
        &self,
        agent_id: &str,
        message: Message,
        stream: bool,
    ) -> RuntimeResult<Message>;

    /// Create a new agent
    async fn create_agent(
        &self,
        definition_id: String,
        session_id: String,
        variant: Option<String>,
    ) -> RuntimeResult<Agent>;

    /// Get an agent by ID
    async fn get_agent(&self, agent_id: &str) -> RuntimeResult<Agent>;

    /// Check if the runtime is initialized
    fn is_initialized(&self) -> bool;
}
