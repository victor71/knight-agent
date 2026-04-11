//! Session Manager
//!
//! Manages all session lifecycle including creation, switching, deletion, and persistence.
//! Also handles workspace isolation, context management, and message history storage.
//!
//! Design Reference: docs/03-module-design/core/session-manager.md

pub mod manager;
pub mod types;

// Re-export AgentRuntimeProxy from agent-proxy for convenience
pub use agent_proxy::{AgentRuntimeProxy, StreamCallback};

// Re-export commonly used types
pub use manager::SessionManagerImpl;
pub use types::{
    CompressionMethod, CreateSessionRequest, Message, PathAction, ProjectType, Session,
    SessionStatus,
};
