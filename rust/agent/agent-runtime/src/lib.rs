//! Agent Runtime
//!
//! Design Reference: docs/03-module-design/agent/agent-runtime.md
//!
//! This module provides the core agent runtime system, including:
//! - Agent lifecycle management (create, start, stop, pause, resume)
//! - Message handling and context management
//! - State machine for agent status transitions
//! - Tool call proxy interface

pub mod runtime;
pub mod types;

pub use types::{
    Agent, AgentContext, AgentRuntimeError, AgentState, AgentStatistics, AgentStatus, AwaitInfo,
    ContentBlock, ContentBlockType, ErrorInfo, MemoryItem, Message, MessageRole, RuntimeResult,
    ToolResult, UserResponse,
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

    /// Get or create a session agent
    /// Returns the agent ID for the session, creating a new one if needed
    async fn get_or_create_session_agent(&self, session_id: String) -> RuntimeResult<String>;

    /// Check if the runtime is initialized
    fn is_initialized(&self) -> bool;
}

// Implement AgentRuntimeProxy for AgentRuntimeImpl
use agent_proxy::{AgentRuntimeProxy as ProxyTrait, StreamCallback};

#[async_trait::async_trait]
impl ProxyTrait for AgentRuntimeImpl {
    async fn get_or_create_session_agent(
        &self,
        session_id: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        AgentRuntimeImpl::get_or_create_session_agent(self, session_id)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn send_message(
        &self,
        agent_id: &str,
        content: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let message = Message::user(content);
        // Use streaming by default for better user experience
        let response = AgentRuntimeImpl::send_message(self, agent_id, message, true)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(response.content.to_string())
    }

    async fn send_message_streaming(
        &self,
        agent_id: &str,
        content: String,
        stream_callback: Option<StreamCallback>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let message = Message::user(content);
        let response = AgentRuntimeImpl::send_message_streaming_with_callback(
            self,
            agent_id,
            message,
            true, // stream parameter
            stream_callback,
        )
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(response.content.to_string())
    }
}
