//! Session Manager Implementation
//!
//! Handles session lifecycle, context management, and workspace isolation.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info};

use crate::types::*;
use agent_proxy::{AgentRuntimeProxy, StreamCallback};

/// Session manager implementation
pub struct SessionManagerImpl {
    sessions: Arc<AsyncRwLock<HashMap<String, Session>>>,
    current_session: Arc<AsyncRwLock<Option<String>>>,
    initialized: Arc<AsyncRwLock<bool>>,
    agent_runtime: Arc<AsyncRwLock<Option<Arc<dyn AgentRuntimeProxy>>>>,
}

impl SessionManagerImpl {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(AsyncRwLock::new(HashMap::new())),
            current_session: Arc::new(AsyncRwLock::new(None)),
            initialized: Arc::new(AsyncRwLock::new(false)),
            agent_runtime: Arc::new(AsyncRwLock::new(None)),
        }
    }

    /// Set the agent runtime
    pub async fn set_agent_runtime(&self, runtime: Arc<dyn AgentRuntimeProxy>) {
        let mut agent_runtime = self.agent_runtime.write().await;
        *agent_runtime = Some(runtime);
        info!("Agent runtime set for session manager");
    }

    /// Check if the manager is initialized
    pub fn is_initialized(&self) -> bool {
        // Use try_read to avoid blocking in sync context
        self.initialized.try_read().map(|g| *g).unwrap_or(false)
    }

    /// Initialize the session manager
    pub async fn initialize(&self) -> SessionResult<()> {
        let mut initialized = self.initialized.write().await;
        *initialized = true;
        info!("Session manager initialized");
        Ok(())
    }

    /// Create a new session
    pub async fn create_session(&self, request: CreateSessionRequest) -> SessionResult<Session> {
        let id = generate_session_id();
        let workspace = request.workspace;

        let metadata = SessionMetadata {
            name: request.name.unwrap_or_else(|| format!("session-{}", &id[..8])),
            workspace: workspace.clone(),
            project_type: request.project_type.unwrap_or(ProjectType::Auto),
            description: request.description.unwrap_or_default(),
            tags: request.tags.unwrap_or_default(),
        };

        let session = Session::new(id.clone(), metadata);

        // Check if already exists
        {
            let sessions = self.sessions.read().await;
            if sessions.contains_key(&id) {
                return Err(SessionError::AlreadyExists(id));
            }
        }

        // Insert
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(id.clone(), session.clone());
        }

        // Set as current if first session
        {
            let mut current = self.current_session.write().await;
            if current.is_none() {
                *current = Some(id.clone());
            }
        }

        info!("Created session: {} in workspace: {}", id, workspace);
        Ok(session)
    }

    /// Get a session by ID
    pub async fn get_session(&self, id: &str) -> SessionResult<Session> {
        let sessions = self.sessions.read().await;
        sessions
            .get(id)
            .cloned()
            .ok_or_else(|| SessionError::NotFound(id.to_string()))
    }

    /// List all sessions, optionally filtered by status
    pub async fn list_sessions(&self, status_filter: Option<SessionStatus>) -> Vec<Session> {
        let sessions = self.sessions.read().await;

        sessions
            .values()
            .filter(|s| {
                if let Some(status) = status_filter {
                    s.status == status
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Delete a session
    pub async fn delete_session(&self, id: &str, _force: bool) -> SessionResult<()> {
        let removed = {
            let mut sessions = self.sessions.write().await;
            sessions.remove(id)
        };

        if removed.is_none() {
            return Err(SessionError::NotFound(id.to_string()));
        }

        // Clear current if was current
        {
            let mut current = self.current_session.write().await;
            if *current == Some(id.to_string()) {
                *current = None;
            }
        }

        info!("Deleted session: {}", id);
        Ok(())
    }

    /// Archive a session
    pub async fn archive_session(&self, id: &str) -> SessionResult<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(id) {
            session.status = SessionStatus::Archived;
            session.updated_at = chrono::Utc::now().to_rfc3339();
            info!("Archived session: {}", id);
            Ok(())
        } else {
            Err(SessionError::NotFound(id.to_string()))
        }
    }

    /// Restore an archived session
    pub async fn restore_session(&self, id: &str) -> SessionResult<Session> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(id) {
            if session.status != SessionStatus::Archived {
                return Err(SessionError::InvalidState(
                    "Can only restore archived sessions".to_string(),
                ));
            }
            session.status = SessionStatus::Active;
            session.updated_at = chrono::Utc::now().to_rfc3339();
            info!("Restored session: {}", id);
            Ok(session.clone())
        } else {
            Err(SessionError::NotFound(id.to_string()))
        }
    }

    /// Switch to a different session
    pub async fn use_session(&self, id: &str) -> SessionResult<Session> {
        // Verify session exists
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(id).cloned()
        };

        let session = session.ok_or_else(|| SessionError::NotFound(id.to_string()))?;

        {
            let mut current = self.current_session.write().await;
            *current = Some(id.to_string());
        }

        info!("Switched to session: {}", id);
        Ok(session)
    }

    /// Get the current active session
    pub async fn get_current_session(&self) -> Option<Session> {
        let current = self.current_session.read().await;
        let current_id = current.as_ref()?.clone();
        drop(current);

        let sessions = self.sessions.read().await;
        sessions.get(&current_id).cloned()
    }

    /// Add a message to a session's context
    pub async fn add_message(&self, session_id: &str, message: Message) -> SessionResult<bool> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.context.add_message(message.clone());
            session.stats.total_messages += 1;
            session.last_active = Some(chrono::Utc::now().to_rfc3339());
            session.updated_at = chrono::Utc::now().to_rfc3339();

            debug!("Added message to session {}: total messages = {}", session_id, session.stats.total_messages);

            // Return true if compression should be triggered (e.g., > 100 messages)
            let should_compress = session.stats.total_messages > 100;
            Ok(should_compress)
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Get session context
    pub async fn get_context(&self, session_id: &str, _include_compressed: bool) -> SessionResult<SessionContext> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let ctx = session.context.clone();
            // Note: filtering based on include_compressed would be implemented here
            Ok(ctx)
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Compress session context
    pub async fn compress_context(
        &self,
        session_id: &str,
        method: CompressionMethod,
    ) -> SessionResult<CompressionPoint> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            let original_count = session.context.messages.len() as u32;

            // Create a compression point
            let point = CompressionPoint {
                id: generate_id(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                method,
                original_count,
                compressed_count: original_count / 2, // Simulated compression
                summary: format!("Compressed {} messages", original_count),
                token_saved: original_count * 10, // Simulated
            };

            session.context.add_compression_point(point.clone());
            session.stats.compression_count += 1;
            session.stats.total_tokens = session.stats.total_tokens.saturating_sub(point.token_saved as u64);
            session.updated_at = chrono::Utc::now().to_rfc3339();

            info!(
                "Compressed session {} context: {} -> {} messages",
                session_id, original_count, point.compressed_count
            );

            Ok(point)
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Search session history
    pub async fn search_history(
        &self,
        query: &str,
        session_id: Option<&str>,
        limit: usize,
    ) -> SessionResult<Vec<SearchResult>> {
        let sessions = self.sessions.read().await;

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for session in sessions.values() {
            // Filter by session_id if provided
            if let Some(sid) = session_id {
                if session.id != sid {
                    continue;
                }
            }

            // Search in messages
            for msg in &session.context.messages {
                if msg.content.to_lowercase().contains(&query_lower) {
                    results.push(SearchResult {
                        session_id: session.id.clone(),
                        message_id: msg.id.clone(),
                        content: msg.content.clone(),
                        relevance_score: 1.0, // Simplified
                        timestamp: msg.timestamp.clone(),
                    });

                    if results.len() >= limit {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Update session context (called by context compressor)
    pub async fn update_context(
        &self,
        session_id: &str,
        compressed_messages: Vec<Message>,
    ) -> SessionResult<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.context.messages = compressed_messages;
            session.updated_at = chrono::Utc::now().to_rfc3339();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Get compression point
    pub async fn get_compression_point(
        &self,
        session_id: &str,
        point_id: &str,
    ) -> SessionResult<CompressionPoint> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            session
                .context
                .compression_points
                .iter()
                .find(|p| p.id == point_id)
                .cloned()
                .ok_or_else(|| SessionError::NotFound(format!("Compression point {} not found", point_id)))
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Check path access permission within a session's workspace
    pub async fn check_path_access(
        &self,
        session_id: &str,
        path: &str,
        _action: PathAction,
    ) -> SessionResult<PathAccessResult> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let workspace = &session.metadata.workspace;

            // Check if path is within workspace
            // Note: On Windows/MINGW, paths like "/etc/passwd" are treated as relative
            // by Path::is_relative(), so we check path prefix directly
            let allowed = if path.is_empty() {
                false
            } else if path.starts_with('/') || path.contains(':') {
                // Looks like an absolute path (Unix or Windows), check against workspace
                path.starts_with(workspace)
            } else {
                // Truly relative path - allow (resolved relative to workspace)
                true
            };

            Ok(PathAccessResult {
                allowed,
                reason: if !allowed {
                    Some("Path outside workspace boundary".to_string())
                } else {
                    None
                },
            })
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Validate path is within workspace
    pub async fn validate_path(&self, session_id: &str, path: &str) -> SessionResult<bool> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let workspace = &session.metadata.workspace;
            let allowed = path.starts_with(workspace);
            Ok(allowed)
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Save session (persist to storage - placeholder for now)
    pub async fn save_session(&self, id: Option<&str>) -> SessionResult<String> {
        let session_id = if let Some(id) = id {
            id.to_string()
        } else {
            let current = self.current_session.read().await;
            current.clone().ok_or(SessionError::NotInitialized)?
        };

        // Verify session exists
        {
            let sessions = self.sessions.read().await;
            if !sessions.contains_key(&session_id) {
                return Err(SessionError::NotFound(session_id));
            }
        }

        // In a real implementation, this would persist to storage
        let path = format!("/sessions/{}", session_id);
        info!("Saved session {} to {}", session_id, path);
        Ok(path)
    }

    /// Load session from storage (placeholder)
    pub async fn load_session(&self, id: &str) -> SessionResult<Session> {
        // In a real implementation, this would load from storage
        self.get_session(id).await
    }

    /// Get session statistics
    pub async fn get_stats(&self, session_id: &str) -> SessionResult<SessionStats> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            Ok(session.stats.clone())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Clear all sessions
    pub async fn clear(&self) {
        let mut sessions = self.sessions.write().await;
        let mut current = self.current_session.write().await;
        sessions.clear();
        *current = None;
        info!("Cleared all sessions");
    }

    /// Get total session count
    pub async fn len(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Check if no sessions exist
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Send message to session's agent
    /// This is the main entry point for UI layer to send messages
    pub async fn send_message_to_session(&self, session_id: &str, content: String) -> SessionResult<String> {
        self.send_message_to_session_streaming(session_id, content, None).await
    }

    /// Send message to session's agent with streaming callback
    pub async fn send_message_to_session_streaming(
        &self,
        session_id: &str,
        content: String,
        stream_callback: Option<StreamCallback>,
    ) -> SessionResult<String> {
        // Verify session exists
        {
            let sessions = self.sessions.read().await;
            if !sessions.contains_key(session_id) {
                return Err(SessionError::NotFound(session_id.to_string()));
            }
        }

        // Get agent runtime
        let agent_runtime = self.agent_runtime.read().await;
        let agent_runtime = agent_runtime.as_ref()
            .ok_or_else(|| SessionError::NotInitialized)?;

        // Get or create agent for this session
        let agent_id = agent_runtime.get_or_create_session_agent(session_id.to_string())
            .await
            .map_err(|e| SessionError::CompressionError(e.to_string()))?;

        // Send message to agent with streaming callback
        let response = agent_runtime.send_message_streaming(&agent_id, content.clone(), stream_callback)
            .await
            .map_err(|e| SessionError::CompressionError(e.to_string()))?;

        // Add user message to session context
        let _ = self.add_message(session_id, Message::user(
            generate_id(),
            format!("user: {}", content)
        )).await;

        // Add assistant response to session context
        let _ = self.add_message(session_id, Message::assistant(
            generate_id(),
            response.clone()
        )).await;

        Ok(response)
    }
}

impl Default for SessionManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    format!("sess_{}", generate_id())
}

/// Generate a unique ID
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}-{:x}", duration.as_secs(), duration.subsec_nanos())
}
