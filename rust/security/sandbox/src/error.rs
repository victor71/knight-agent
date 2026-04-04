use thiserror::Error;

pub type SandboxResult<T> = Result<T, SandboxError>;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Sandbox not initialized")]
    NotInitialized,
    #[error("Sandbox creation failed: {0}")]
    CreationFailed(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("Sandbox not found: {0}")]
    SandboxNotFound(String),
    #[error("Access denied: {0}")]
    AccessDenied(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Violation reported: {0}")]
    Violation(String),
}
