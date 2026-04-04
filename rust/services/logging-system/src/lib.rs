//! Logging System
//!
//! Design Reference: docs/03-module-design/services/logging-system.md
//!
//! A high-performance, structured logging system for Knight-Agent.
//!
//! # Features
//!
//! - Structured JSON logging
//! - Multiple log levels (TRACE, DEBUG, INFO, WARN, ERROR, FATAL)
//! - Async logging for non-blocking operation
//! - Multiple output targets (console, file)
//! - Log filtering and querying
//! - Log statistics
//!
//! # Example
//!
//! ```rust,no_run
//! use logging_system::{LoggingSystemImpl, LoggingSystem};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let logging = LoggingSystemImpl::new()?;
//!     logging.init().await?;
//!
//!     logging.info("Application started".to_string()).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod init;
pub mod output;
pub mod system;
pub mod types;

pub use init::LoggerGuard;
pub use output::LogOutput;
pub use system::{LoggingSystem, LoggingSystemImpl};
pub use types::{
    ErrorInfo, ExportFormat, LogContext, LogEntry, LogFilter, LogLevel, LogStats,
};

#[derive(thiserror::Error, Debug)]
pub enum LoggingError {
    #[error("Logging system not initialized")]
    NotInitialized,
    #[error("Log write failed: {0}")]
    WriteFailed(String),
    #[error("Log rotation failed: {0}")]
    RotationFailed(String),
    #[error("Invalid log level: {0}")]
    InvalidLevel(String),
    #[error("Query failed: {0}")]
    QueryFailed(String),
    #[error("Export failed: {0}")]
    ExportFailed(String),
}
