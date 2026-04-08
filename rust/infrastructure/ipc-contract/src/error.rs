//! IPC Error types

use thiserror::Error;

/// IPC error codes (defined by contract specification)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    // General errors (1-999)
    UnknownError = 1,
    ParseError = 2,
    InvalidRequest = 3,
    MethodNotFound = 4,
    Timeout = 5,

    // Session errors (1000-1999)
    SessionNotFound = 1000,
    SessionExpired = 1001,
    SessionDestroyed = 1002,

    // Agent errors (2000-2999)
    AgentNotFound = 2000,
    AgentSpawnFailed = 2001,
    AgentTimeout = 2002,

    // Tool errors (3000-3999)
    ToolNotFound = 3000,
    ToolExecutionFailed = 3001,

    // Security errors (4000-4999)
    Unauthorized = 4001,
    Forbidden = 4002,
    PermissionDenied = 4003,

    // User interaction errors (6000-6999)
    AwaitTimeout = 6000,
    AwaitCancelled = 6001,
    InvalidUserResponse = 6002,
    AwaitNotFound = 6003,

    // System errors (5000-5999)
    InternalError = 5000,
    ResourceExhausted = 5001,
}

impl ErrorCode {
    /// Get error code as i32
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    /// Get error message
    pub fn message(self) -> &'static str {
        match self {
            ErrorCode::UnknownError => "Unknown error occurred",
            ErrorCode::ParseError => "Failed to parse message",
            ErrorCode::InvalidRequest => "Invalid request format",
            ErrorCode::MethodNotFound => "Method not found",
            ErrorCode::Timeout => "Request timeout",
            ErrorCode::SessionNotFound => "Session not found",
            ErrorCode::SessionExpired => "Session has expired",
            ErrorCode::SessionDestroyed => "Session was destroyed",
            ErrorCode::AgentNotFound => "Agent not found",
            ErrorCode::AgentSpawnFailed => "Failed to spawn agent",
            ErrorCode::AgentTimeout => "Agent operation timeout",
            ErrorCode::ToolNotFound => "Tool not found",
            ErrorCode::ToolExecutionFailed => "Tool execution failed",
            ErrorCode::Unauthorized => "Unauthorized access",
            ErrorCode::Forbidden => "Forbidden operation",
            ErrorCode::PermissionDenied => "Permission denied",
            ErrorCode::AwaitTimeout => "User response timeout",
            ErrorCode::AwaitCancelled => "User query was cancelled",
            ErrorCode::InvalidUserResponse => "Invalid user response format",
            ErrorCode::AwaitNotFound => "Await ID not found",
            ErrorCode::InternalError => "Internal error",
            ErrorCode::ResourceExhausted => "Resource exhausted",
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            ErrorCode::Timeout | ErrorCode::InternalError | ErrorCode::ResourceExhausted
        )
    }
}

/// IPC error type
#[derive(Error, Debug)]
pub enum IPCError {
    #[error("IPC not initialized")]
    NotInitialized,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Timeout: operation exceeded {0}ms")]
    Timeout(u64),
    #[error("Session error: {0}")]
    SessionError(String),
    #[error("Agent error: {0}")]
    AgentError(String),
    #[error("Tool error: {0}")]
    ToolError(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("User interaction error: {0}")]
    UserInteractionError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl IPCError {
    /// Convert to ErrorCode
    pub fn error_code(&self) -> ErrorCode {
        match self {
            IPCError::NotInitialized => ErrorCode::InternalError,
            IPCError::ConnectionFailed(_) => ErrorCode::InternalError,
            IPCError::SendFailed(_) => ErrorCode::InternalError,
            IPCError::ReceiveFailed(_) => ErrorCode::InternalError,
            IPCError::ParseError(_) => ErrorCode::ParseError,
            IPCError::InvalidRequest(_) => ErrorCode::InvalidRequest,
            IPCError::MethodNotFound(_) => ErrorCode::MethodNotFound,
            IPCError::Timeout(_) => ErrorCode::Timeout,
            IPCError::SessionError(_) => ErrorCode::SessionNotFound,
            IPCError::AgentError(_) => ErrorCode::AgentNotFound,
            IPCError::ToolError(_) => ErrorCode::ToolExecutionFailed,
            IPCError::Unauthorized(_) => ErrorCode::Unauthorized,
            IPCError::PermissionDenied(_) => ErrorCode::PermissionDenied,
            IPCError::UserInteractionError(_) => ErrorCode::AwaitNotFound,
            IPCError::InternalError(_) => ErrorCode::InternalError,
            IPCError::JsonError(_) => ErrorCode::ParseError,
            IPCError::IoError(_) => ErrorCode::InternalError,
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        self.error_code().is_retryable()
    }
}

pub type IPCResult<T> = Result<T, IPCError>;
