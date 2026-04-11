//! Permission Service

// Re-export public API
pub use error::{PermissionError, PermissionResult};
pub use r#trait::PermissionService;
pub use service::PermissionServiceImpl;
pub use types::Permission;

mod error;
mod service;
mod r#trait;
mod types;
