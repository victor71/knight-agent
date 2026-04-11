//! Context Compressor
//!
//! Intelligent context compression for conversation management.
//!
//! Design Reference: docs/03-module-design/services/context-compressor.md

// Re-export public API
pub use api::ContextCompressor;
pub use compressor::ContextCompressorImpl;
pub use content::ContentType;
pub use error::{ContextCompressorError, ContextCompressorResult};
pub use types::{
    CompressedContext, CompressionConfig, CompressionHistoryEntry, CompressionJob,
    CompressionJobStatus, CompressionOptions, CompressionPoint, CompressionStats,
    CompressionStrategy, CompressionTrigger, Message, MessageRange, ShouldCompressResponse,
    TokenEstimation,
};

mod api;
mod compressor;
mod content;
mod error;
mod types;
