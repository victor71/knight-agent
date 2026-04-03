//! LLM Provider
//!
//! Design Reference: docs/03-module-design/services/llm-provider.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMProviderError {
    #[error("LLM provider not initialized")]
    NotInitialized,
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub model: String,
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub trait LLMProvider: Send + Sync {
    fn new() -> Result<Self, LLMProviderError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMProviderError>;
    async fn list_models(&self) -> Result<Vec<String>, LLMProviderError>;
}

pub struct LLMProviderImpl;

impl LLMProvider for LLMProviderImpl {
    fn new() -> Result<Self, LLMProviderError> {
        Ok(LLMProviderImpl)
    }

    fn name(&self) -> &str {
        "llm-provider"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn complete(&self, _request: LLMRequest) -> Result<LLMResponse, LLMProviderError> {
        Ok(LLMResponse {
            content: String::new(),
            model: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
        })
    }

    async fn list_models(&self) -> Result<Vec<String>, LLMProviderError> {
        Ok(vec![])
    }
}
