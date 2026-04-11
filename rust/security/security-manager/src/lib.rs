//! Security Manager
//!
//! Design Reference: docs/03-module-design/security/security-manager.md
//!
//! A comprehensive security management system for Knight-Agent.
//!
//! # Features
//!
//! - Permission management (grant, revoke, check)
//! - Security policy engine (RBAC, ABAC)
//! - Audit logging and event tracking
//! - Secret/key management
//! - Threat detection
//!
//! # Example
//!
//! ```rust,no_run
//! use security_manager::{SecurityManagerImpl, SecurityManager};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let security = SecurityManagerImpl::new()?;
//!     security.init().await?;
//!
//!     let (allowed, reason) = security
//!         .check_permission("user:admin", "file:/project/**", "read", None)
//!         .await?;
//!
//!     println!("Permission allowed: {}, reason: {:?}", allowed, reason);
//!     Ok(())
//! }
//! ```

pub mod audit;
pub mod policy;
pub mod secrets;
pub mod system;
pub mod types;

pub use audit::AuditLogger;
pub use policy::PolicyEngine;
pub use secrets::SecretManager;
pub use system::{SecurityManager, SecurityManagerImpl};
pub use types::*;

#[derive(thiserror::Error, Debug)]
pub enum SecurityError {
    #[error("Security manager not initialized")]
    NotInitialized,

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Authorization failed: {0}")]
    AuthzFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Secret error: {0}")]
    SecretError(String),

    #[error("Threat detected: {0}")]
    ThreatDetected(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
