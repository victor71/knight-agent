//! CLI error types

use thiserror::Error;

/// CLI error type
#[derive(Error, Debug)]
pub enum CliError {
    /// Daemon is not running
    #[error("Daemon not running")]
    DaemonNotRunning,
    /// Connection to daemon failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    /// Command not found
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    /// Operation timed out
    #[error("Timeout")]
    Timeout,
    /// IPC communication error
    #[error("IPC error: {0}")]
    IpcError(String),
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    /// CLI not initialized
    #[error("CLI not initialized")]
    NotInitialized,
    /// Generic error
    #[error("Error: {0}")]
    Other(String),
}

/// Result type for CLI operations
pub type CliResult<T> = Result<T, CliError>;
