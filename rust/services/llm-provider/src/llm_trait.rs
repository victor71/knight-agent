//! LLM Provider Trait
//!
//! Core trait for LLM provider abstraction.

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::types::*;

/// Token count result
#[derive(Debug, Clone)]
pub struct TokenCount {
    pub count: usize,
}

/// LLM Provider errors
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("provider not initialized")]
    NotInitialized,
    #[error("inference failed: {0}")]
    InferenceFailed(String),
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("provider not found: {0}")]
    ProviderNotFound(String),
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("context length exceeded")]
    ContextLengthExceeded,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("timeout")]
    Timeout,
    #[error("api key invalid")]
    ApiKeyInvalid,
}

/// Result type for LLM operations
pub type LLMResult<T> = Result<T, LLMError>;

/// Streaming completion response
pub type CompletionStream = Pin<Box<dyn Stream<Item = LLMResult<ChatCompletionChunk>> + Send>>;

/// LLM Provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Create a new provider instance
    fn new() -> LLMResult<Self>
    where
        Self: Sized;

    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is initialized
    fn is_initialized(&self) -> bool;

    /// Chat completion (non-streaming)
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<ChatCompletionResponse>;

    /// Streaming chat completion
    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<CompletionStream>;

    /// Count tokens for a given text
    async fn count_tokens(&self, text: &str, model: &str) -> LLMResult<TokenCount>;

    /// Estimate cost for a request
    async fn estimate_cost(&self, request: &ChatCompletionRequest) -> LLMResult<CostEstimate>;

    /// List available models
    async fn list_models(&self) -> LLMResult<Vec<String>>;

    /// Get model information
    async fn get_model_info(&self, model: &str) -> LLMResult<ModelInfo>;

    /// Health check for the provider
    async fn health_check(&self) -> LLMResult<ProviderStatus>;
}
