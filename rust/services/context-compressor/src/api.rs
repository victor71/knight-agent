//! Context compressor trait definition

use crate::error::ContextCompressorResult;
use crate::types::{
    CompressionConfig, CompressionJob, CompressionOptions, CompressionPoint,
    CompressionStats, CompressionStrategy, Message, ShouldCompressResponse,
    TokenEstimation,
};

/// Context compressor trait
pub trait ContextCompressor: Send + Sync {
    fn new() -> Result<Self, ContextCompressorError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn initialize(&self) -> ContextCompressorResult<()>;
    async fn should_compress(&self, session_id: &str) -> ContextCompressorResult<ShouldCompressResponse>;
    async fn compress(
        &self,
        session_id: &str,
        messages: Vec<Message>,
        strategy: CompressionStrategy,
        options: CompressionOptions,
    ) -> ContextCompressorResult<CompressionPoint>;
    async fn compress_async(
        &self,
        session_id: &str,
        messages: Vec<Message>,
        strategy: CompressionStrategy,
    ) -> ContextCompressorResult<String>;
    async fn get_compression_job_status(&self, job_id: &str) -> ContextCompressorResult<CompressionJob>;
    async fn get_compression_points(&self, session_id: &str) -> ContextCompressorResult<Vec<CompressionPoint>>;
    async fn get_compression_point(&self, point_id: &str) -> ContextCompressorResult<CompressionPoint>;
    async fn restore_compression_point(&self, point_id: &str) -> ContextCompressorResult<Vec<Message>>;
    async fn delete_compression_point(&self, point_id: &str) -> ContextCompressorResult<()>;
    async fn estimate_tokens(&self, messages: &[Message]) -> ContextCompressorResult<TokenEstimation>;
    async fn get_compression_stats(&self, session_id: &str) -> ContextCompressorResult<CompressionStats>;
    async fn get_compression_config(&self, session_id: Option<&str>) -> ContextCompressorResult<CompressionConfig>;
    async fn update_compression_config(&self, config: CompressionConfig) -> ContextCompressorResult<()>;
}

// Re-export error for the trait
use crate::error::ContextCompressorError;
