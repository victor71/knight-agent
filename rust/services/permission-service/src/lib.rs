//! Permission Service

// Re-export public API
pub use error::{PermissionError, PermissionResult};
pub use service::PermissionServiceImpl;
pub use r#trait::PermissionService;
pub use types::Permission;

mod error;
mod r#trait;
mod types;
mod service;
