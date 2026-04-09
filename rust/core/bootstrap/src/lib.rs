//! Bootstrap - 8 Stage Initialization System
//!
//! System startup orchestrator for Knight-Agent.
//! Manages the initialization of all 23 modules in 8 stages.
//!
//! Design Reference: docs/03-module-design/core/bootstrap.md

// Re-export public API
pub use error::{BootstrapError, BootstrapResult};
pub use system::{KnightAgentSystem, SystemHandle, SystemHandleImpl};
pub use types::{
    BootstrapConfig, BootstrapMode, BootstrapStage, HealthCheckResult, ModuleHealth, ModuleStatus,
    SystemStatus, VersionInfo,
};

mod error;
mod system;
mod types;
