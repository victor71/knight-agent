//! LLM Provider Module
//!
//! Unified LLM provider interface supporting Anthropic and OpenAI protocols.
//! Configuration-driven design allows flexible provider setup.
//!
//! # Features
//!
//! - Unified chat completion interface
//! - Streaming response support
//! - OpenAI and Anthropic protocol support
//! - Configuration-driven provider setup
//! - Token counting and cost estimation
//! - Model routing and fallback
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use llm_provider::{LLMProvider, ChatCompletionRequest, Message, MessageRole, Content};
//! use llm_provider::provider::{GenericLLMProvider, LLMProtocol, ProviderConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create OpenAI-compatible provider
//!     let openai_config = ProviderConfig {
//!         name: "openai".to_string(),
//!         api_key: "your-api-key".to_string(),
//!         base_url: "https://api.openai.com/v1".to_string(),
//!         protocol: LLMProtocol::OpenAI,
//!         models: vec!["gpt-4o".to_string()],
//!         default_model: Some("gpt-4o".to_string()),
//!         timeout_secs: 120,
//!     };
//!     let provider = GenericLLMProvider::new(openai_config)?;
//!
//!     let request = ChatCompletionRequest {
//!         model: "gpt-4o".to_string(),
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
pub mod router;
pub mod types;

pub use provider::{GenericLLMProvider, LLMProtocol, ProviderConfig};
pub use llm_trait::{LLMProvider, LLMError, LLMResult, TokenCount, CompletionStream};
pub use router::LLMRouter;
pub use types::*;
