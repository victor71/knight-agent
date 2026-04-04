//! CLI error types

use thiserror::Error;

/// CLI error type
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Daemon not running")]
    DaemonNotRunning,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Timeout")]
    Timeout,
    #[error("IPC error: {0}")]
    IpcError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("CLI not initialized")]
    NotInitialized,
}

pub type CliResult<T> = Result<T, CliError>;
