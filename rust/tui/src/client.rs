//! Daemon Client
//!
//! Abstraction layer for TUI to communicate with daemon.

pub mod ipc;

use async_trait::async_trait;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tokio::sync::mpsc;

use crate::event::{AppEvent, SystemStatusSnapshot};
use crate::state::SessionListItem;

/// Result type for daemon client operations
pub type DaemonClientResult<T> = Result<T, DaemonClientError>;

/// Daemon client errors
#[derive(Debug, thiserror::Error)]
pub enum DaemonClientError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Timeout: operation exceeded {0}ms")]
    Timeout(u64),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Not connected")]
    NotConnected,
}

/// Daemon client trait for TUI to communicate with daemon
#[async_trait]
pub trait DaemonClient: Send + Sync {
    /// Handle user input (check if command, route appropriately)
    fn handle_input(&self, input: String, session_id: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<router::HandleInputResult>> + Send>>;

    /// Send message to agent in session
    fn send_message(&self, session_id: &str, content: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>>;

    /// List all sessions
    fn list_sessions(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<Vec<SessionListItem>>> + Send>>;

    /// Create a new session
    fn create_session(&self, name: Option<String>, workspace: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>>;

    /// Switch to a different session
    fn switch_session(&self, session_id: &str) -> Pin<Box<dyn Future<Output = DaemonClientResult<()>> + Send>>;

    /// Get current system status
    fn get_system_status(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<SystemStatusSnapshot>> + Send>>;

    /// Subscribe to daemon events
    fn subscribe_events(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<mpsc::UnboundedReceiver<AppEvent>>> + Send>>;
}

/// Direct daemon client - wraps Arc references to router and session manager
/// This is used in the current single-process mode
pub struct DirectDaemonClient {
    router: Option<Arc<dyn router::RouterHandle>>,
    session_manager: Option<Arc<session_manager::SessionManagerImpl>>,
}

impl DirectDaemonClient {
    /// Create a new direct daemon client
    pub fn new() -> Self {
        Self {
            router: None,
            session_manager: None,
        }
    }

    /// Set the router handle
    pub fn with_router(mut self, router: Arc<dyn router::RouterHandle>) -> Self {
        self.router = Some(router);
        self
    }

    /// Set the session manager
    pub fn with_session_manager(mut self, session_manager: Arc<session_manager::SessionManagerImpl>) -> Self {
        self.session_manager = Some(session_manager);
        self
    }

    /// Check if connected (has required components)
    pub fn is_connected(&self) -> bool {
        self.router.is_some() && self.session_manager.is_some()
    }
}

impl Default for DirectDaemonClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DaemonClient for DirectDaemonClient {
    fn handle_input(&self, input: String, session_id: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<router::HandleInputResult>> + Send>> {
        let router = match &self.router {
            Some(r) => r.clone(),
            None => return Box::pin(async { Err(DaemonClientError::NotConnected) }),
        };

        Box::pin(async move {
            // RouterHandle::handle_input takes (input, session_id) directly
            let result = router.handle_input(input, session_id).await;

            // HandleResult is returned directly, not wrapped in Result
            Ok(result)
        })
    }

    fn send_message(&self, session_id: &str, content: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>> {
        let session_mgr = match &self.session_manager {
            Some(s) => s.clone(),
            None => return Box::pin(async { Err(DaemonClientError::NotConnected) }),
        };
        let session_id = session_id.to_string();

        Box::pin(async move {
            session_mgr.send_message_to_session(&session_id, content).await
                .map_err(|e| DaemonClientError::SessionError(e.to_string()))
        })
    }

    fn list_sessions(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<Vec<SessionListItem>>> + Send>> {
        let session_mgr = match &self.session_manager {
            Some(s) => s.clone(),
            None => return Box::pin(async { Err(DaemonClientError::NotConnected) }),
        };

        Box::pin(async move {
            let sessions = session_mgr.list_sessions(None).await;
            Ok(sessions.into_iter().map(|s| {
                let name = if s.metadata.name.is_empty() {
                    s.id.clone()
                } else {
                    s.metadata.name.clone()
                };
                // Parse created_at from ISO string, fallback to current time
                let created_at = s.created_at.parse::<chrono::DateTime<chrono::Utc>>()
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Local))
                    .unwrap_or_else(chrono::Local::now);

                SessionListItem {
                    id: s.id,
                    name,
                    status: format!("{:?}", s.status),
                    created_at,
                    message_count: s.stats.total_messages as usize,
                }
            }).collect())
        })
    }

    fn create_session(&self, name: Option<String>, workspace: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>> {
        let session_mgr = match &self.session_manager {
            Some(s) => s.clone(),
            None => return Box::pin(async { Err(DaemonClientError::NotConnected) }),
        };

        Box::pin(async move {
            // Use builder pattern for CreateSessionRequest
            let mut request = session_manager::CreateSessionRequest::new(workspace);
            if let Some(name) = name {
                request = request.name(name);
            }

            let session = session_mgr.create_session(request).await
                .map_err(|e| DaemonClientError::SessionError(e.to_string()))?;

            Ok(session.id)
        })
    }

    fn switch_session(&self, session_id: &str) -> Pin<Box<dyn Future<Output = DaemonClientResult<()>> + Send>> {
        let session_mgr = match &self.session_manager {
            Some(s) => s.clone(),
            None => return Box::pin(async { Err(DaemonClientError::NotConnected) }),
        };
        let session_id = session_id.to_string();

        Box::pin(async move {
            session_mgr.use_session(&session_id).await
                .map_err(|e| DaemonClientError::SessionError(e.to_string()))?;

            Ok(())
        })
    }

    fn get_system_status(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<SystemStatusSnapshot>> + Send>> {
        // Return default status for now
        Box::pin(async { Ok(SystemStatusSnapshot::default()) })
    }

    fn subscribe_events(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<mpsc::UnboundedReceiver<AppEvent>>> + Send>> {
        // Return empty channel for now - in Phase 4 this will connect to IPC events
        Box::pin(async {
            let (_tx, rx) = mpsc::unbounded_channel();
            Ok(rx)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_client_creation() {
        let client = DirectDaemonClient::new();
        assert!(!client.is_connected());
    }
}
