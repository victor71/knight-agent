//! Error types for permission service

use thiserror::Error;

/// Permission error type
#[derive(Error, Debug)]
pub enum PermissionError {
    #[error("Permission service not initialized")]
    NotInitialized,
    #[error("Permission denied: {0}")]
    Denied(String),
    #[error("Permission check failed: {0}")]
    CheckFailed(String),
}

pub type PermissionResult<T> = Result<T, PermissionError>;
