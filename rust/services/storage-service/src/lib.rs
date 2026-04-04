//! Storage Service
//!
//! Design Reference: docs/03-module-design/services/storage-service.md
//!
//! A SQLite-based storage service providing unified data persistence interface.
//!
//! # Features
//!
//! - Session data storage and retrieval
//! - Message history persistence
//! - Compression point storage
//! - Task state management
//! - Workflow storage
//! - Configuration storage
//! - Statistics persistence
//! - Data backup and restore
//!
//! # Example
//!
//! ```rust,no_run
//! use storage_service::{StorageServiceImpl, StorageService, Session, SessionStatus};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let storage = StorageServiceImpl::new()?;
//!     storage.init().await?;
//!
//!     let now = std::time::SystemTime::now()
//!         .duration_since(std::time::UNIX_EPOCH)?
//!         .as_secs() as i64;
//!
//!     let session = Session {
//!         id: "session-1".to_string(),
//!         name: "Test Session".to_string(),
//!         status: SessionStatus::Active,
//!         workspace_root: "/workspace".to_string(),
//!         project_type: None,
//!         created_at: now,
//!         last_active_at: now,
//!         metadata: Default::default(),
//!     };
//!
//!     storage.save_session(session).await?;
//!     Ok(())
//! }
//! ```

pub mod database;
pub mod system;
pub mod types;

pub use database::StorageError;
pub use system::{StorageService, StorageServiceImpl};
pub use types::*;
