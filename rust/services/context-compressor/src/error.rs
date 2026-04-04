//! Error types for context compressor

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContextCompressorError {
    #[error("Context compressor not initialized")]
    NotInitialized,
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
    #[error("Compression point not found: {0}")]
    PointNotFound(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Compression retry exhausted")]
    CompressionRetryExhausted,
    #[error("Compression token limit exceeded")]
    CompressionTokenLimit,
    #[error("Compression conflict: another compression is in progress")]
    CompressionConflict,
    #[error("LLM service unavailable: {0}")]
    LlmUnavailable(String),
}

pub type ContextCompressorResult<T> = Result<T, ContextCompressorError>;
