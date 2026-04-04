//! Sandbox Module
//!
//! Provides resource isolation and security boundaries for agent operations.
//!
//! Design Reference: docs/03-module-design/security/sandbox.md

// Re-export all public types and functions from modules
pub mod error;
pub mod types;
pub mod checker;
pub mod r#trait;
pub mod sandbox;

// Re-export commonly used types at crate root
pub use error::{SandboxError, SandboxResult};
pub use types::{
    SandboxLevel, FileAction, SandboxStatus, ViolationType, ViolationSeverity,
    Violation, FilesystemSandbox, CommandSandbox, NetworkSandbox, PortRange,
    ResourceLimits, SandboxConfig, ViolationAction, ResourceUsage,
    SandboxInfo, AccessCheckResult,
};
pub use checker::{PermissionChecker, glob_match};
pub use r#trait::Sandbox;
pub use sandbox::SandboxImpl;
