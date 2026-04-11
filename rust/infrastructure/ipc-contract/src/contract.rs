//! IPC Contract trait definition

use crate::error::IPCResult;
use crate::types::{
    NotificationMessage, PendingQuery, RequestMessage, ResponseMessage, UserQueryMessage,
    UserResponseData,
};
use async_trait::async_trait;

/// IPC Contract trait for communication between Rust core and TypeScript UI
#[async_trait]
pub trait IPCContract: Send + Sync {
    /// Create a new IPC contract instance
    fn new() -> Result<Self, crate::error::IPCError>
    where
        Self: Sized;

    /// Get the name of this IPC implementation
    fn name(&self) -> &str;

    /// Check if the IPC layer is initialized
    fn is_initialized(&self) -> bool;

    /// Initialize the IPC layer
    async fn initialize(&self) -> IPCResult<()>;

    /// Connect to the IPC transport
    async fn connect(&self) -> IPCResult<()>;

    /// Disconnect from the IPC transport
    async fn disconnect(&self) -> IPCResult<()>;

    /// Send a request and wait for response
    async fn send_request(&self, request: RequestMessage) -> IPCResult<ResponseMessage>;

    /// Send a notification (fire and forget)
    async fn send_notification(&self, notification: NotificationMessage) -> IPCResult<()>;

    /// Subscribe to stream chunks for a request
    async fn subscribe_stream(&self, request_id: String) -> IPCResult<()>;

    /// Unsubscribe from stream
    async fn unsubscribe_stream(&self, request_id: String) -> IPCResult<()>;

    /// Send a user query (from agent to UI)
    async fn send_user_query(&self, query: UserQueryMessage) -> IPCResult<String>;

    /// Send a user response (from UI to agent)
    async fn send_user_response(
        &self,
        await_id: String,
        response: UserResponseData,
    ) -> IPCResult<()>;

    /// Cancel a user query
    async fn cancel_user_query(&self, await_id: String) -> IPCResult<()>;

    /// List pending user queries
    async fn list_pending_queries(
        &self,
        session_id: Option<String>,
    ) -> IPCResult<Vec<PendingQuery>>;

    /// Handle incoming message (called by transport layer)
    async fn handle_message(&self, data: &[u8]) -> IPCResult<()>;

    /// Get next message (for transport layer to send)
    async fn next_message(&self) -> IPCResult<Vec<u8>>;
}
