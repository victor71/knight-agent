//! Session Manager
//!
//! Manages all session lifecycle including creation, switching, deletion, and persistence.
//! Also handles workspace isolation, context management, and message history storage.
//!
//! Design Reference: docs/03-module-design/core/session-manager.md

pub mod types;
pub mod manager;

pub use types::*;
pub use manager::{SessionManagerImpl, AgentRuntimeProxy};
