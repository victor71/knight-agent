//! Session Manager Types
//!
//! Core data types for the session management system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Session manager errors
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session already exists: {0}")]
    AlreadyExists(String),
    #[error("Session expired")]
    Expired,
    #[error("Session not initialized")]
    NotInitialized,
    #[error("Invalid session state: {0}")]
    InvalidState(String),
    #[error("Persistence error: {0}")]
    PersistenceError(String),
    #[error("Compression error: {0}")]
    CompressionError(String),
}

/// Result type for session operations
pub type SessionResult<T> = Result<T, SessionError>;

/// Session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Archived,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Project type for auto-detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Java,
    Web,
    Other,
    Auto,
}

impl Default for ProjectType {
    fn default() -> Self {
        Self::Auto
    }
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Rust => "rust",
            ProjectType::Node => "node",
            ProjectType::Python => "python",
            ProjectType::Go => "go",
            ProjectType::Java => "java",
            ProjectType::Web => "web",
            ProjectType::Other => "other",
            ProjectType::Auto => "auto",
        }
    }
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub name: String,
    pub workspace: String,
    pub project_type: ProjectType,
    pub description: String,
    pub tags: Vec<String>,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            workspace: String::new(),
            project_type: ProjectType::Auto,
            description: String::new(),
            tags: Vec::new(),
        }
    }
}

/// Session statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_messages: u64,
    pub total_tokens: u64,
    pub compression_count: u32,
    pub last_activity: Option<String>,
}

/// Session context for LLM
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionContext {
    pub messages: Vec<Message>,
    pub variables: HashMap<String, serde_json::Value>,
    pub compression_points: Vec<CompressionPoint>,
}

impl SessionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn set_variable(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.variables.insert(key.into(), value);
    }

    pub fn add_compression_point(&mut self, point: CompressionPoint) {
        self.compression_points.push(point);
    }
}

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl Message {
    pub fn new(id: String, role: MessageRole, content: String) -> Self {
        Self {
            id,
            role,
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    pub fn user(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(id.into(), MessageRole::User, content.into())
    }

    pub fn assistant(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(id.into(), MessageRole::Assistant, content.into())
    }

    pub fn system(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(id.into(), MessageRole::System, content.into())
    }
}

/// Compression point (record of context compression)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionPoint {
    pub id: String,
    pub timestamp: String,
    pub method: CompressionMethod,
    pub original_count: u32,
    pub compressed_count: u32,
    pub summary: String,
    pub token_saved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionMethod {
    Summary,
    Semantic,
    Hybrid,
}

impl Default for CompressionMethod {
    fn default() -> Self {
        Self::Summary
    }
}

/// Search result in session history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
    pub relevance_score: f32,
    pub timestamp: String,
}

/// File index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndexEntry {
    pub path: String,
    pub file_type: String,
    pub size: u64,
    pub modified: String,
    pub indexed_at: String,
}

/// Session - the main session entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub metadata: SessionMetadata,
    pub status: SessionStatus,
    pub context: SessionContext,
    pub stats: SessionStats,
    pub created_at: String,
    pub updated_at: String,
    pub last_active: Option<String>,
}

impl Session {
    pub fn new(id: String, metadata: SessionMetadata) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            metadata,
            status: SessionStatus::Active,
            context: SessionContext::default(),
            stats: SessionStats::default(),
            created_at: now.clone(),
            updated_at: now,
            last_active: None,
        }
    }

    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.context.messages = messages;
        self
    }
}

/// Session creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub name: Option<String>,
    pub workspace: String,
    pub project_type: Option<ProjectType>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl CreateSessionRequest {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            name: None,
            workspace: workspace.into(),
            project_type: None,
            description: None,
            tags: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn project_type(mut self, project_type: ProjectType) -> Self {
        self.project_type = Some(project_type);
        self
    }
}

/// Path access request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathAction {
    Read,
    Write,
    Execute,
}

/// Path access result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathAccessResult {
    pub allowed: bool,
    pub reason: Option<String>,
}
