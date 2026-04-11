//! Sandbox Module
//!
//! Provides resource isolation and security boundaries for agent operations.
//!
//! Design Reference: docs/03-module-design/security/sandbox.md

// Re-export all public types and functions from modules
pub mod checker;
pub mod error;
pub mod sandbox;
pub mod r#trait;
pub mod types;

// Re-export commonly used types at crate root
pub use checker::{glob_match, PermissionChecker};
pub use error::{SandboxError, SandboxResult};
pub use r#trait::Sandbox;
pub use sandbox::SandboxImpl;
pub use types::{
    AccessCheckResult, CommandSandbox, FileAction, FilesystemSandbox, NetworkSandbox, PortRange,
    ResourceLimits, ResourceUsage, SandboxConfig, SandboxInfo, SandboxLevel, SandboxStatus,
    Violation, ViolationAction, ViolationSeverity, ViolationType,
};
