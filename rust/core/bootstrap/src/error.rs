//! Bootstrap error types

use thiserror::Error;

/// Bootstrap error type
#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Bootstrap failed: {0}")]
    Failed(String),
    #[error("Module initialization failed: {module} - {reason}")]
    ModuleInitFailed { module: String, reason: String },
    #[error("Stage {stage} failed: {reason}")]
    StageFailed { stage: u8, reason: String },
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Timeout: operation exceeded {0}ms")]
    Timeout(u64),
    #[error("Already initialized")]
    AlreadyInitialized,
    #[error("Not initialized")]
    NotInitialized,
}

pub type BootstrapResult<T> = Result<T, BootstrapError>;
