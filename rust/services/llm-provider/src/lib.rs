//! LLM Provider Module
//!
//! Unified LLM provider interface supporting Anthropic and OpenAI.
//!
//! # Features
//!
//! - Unified chat completion interface
//! - Streaming response support
//! - Multiple provider support (Anthropic, OpenAI)
//! - Token counting and cost estimation
//! - Model routing and fallback
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use llm_provider::{LLMProvider, ChatCompletionRequest, Message, MessageRole, Content};
//! use llm_provider::provider::AnthropicProvider;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = AnthropicProvider::new("your-api-key")?;
//!
//!     let request = ChatCompletionRequest {
//!         model: "claude-sonnet-4-6".to_string(),
//!         messages: vec![Message {
//!             role: MessageRole::User,
//!             content: Some(Content::Text("Hello!".to_string())),
//!             tool_calls: None,
//!             tool_call_id: None,
//!         }],
//!         temperature: 0.7,
//!         max_tokens: 1024,
//!         ..Default::default()
//!     };
//!
//!     let response = provider.chat_completion(request).await?;
//!     println!("Response: {}", response.content.unwrap());
//!
//!     Ok(())
//! }
//! ```

pub mod provider;
pub mod llm_trait;
pub mod types;

pub use provider::{AnthropicProvider, OpenAIProvider};
pub use llm_trait::{LLMProvider, LLMError, LLMResult, TokenCount, CompletionStream};
pub use types::*;
