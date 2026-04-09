//! Agent Proxy
//!
//! Shared trait for agent runtime proxy implementations.

use async_trait::async_trait;

/// Stream callback type for streaming LLM responses
pub type StreamCallback = Box<dyn Fn(String) -> bool + Send + Sync>;

/// Agent runtime proxy trait
/// Implemented by agent runtime to provide session management capabilities
#[async_trait]
pub trait AgentRuntimeProxy: Send + Sync {
    /// Get or create a session agent
    async fn get_or_create_session_agent(
        &self,
        session_id: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Send a message to an agent
    async fn send_message(
        &self,
        agent_id: &str,
        content: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Send a message to an agent with streaming callback
    /// The callback is called with each chunk; return false to stop streaming
    async fn send_message_streaming(
        &self,
        agent_id: &str,
        content: String,
        stream_callback: Option<StreamCallback>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}
