//! LLM Provider
//!
//! A configurable LLM provider that supports OpenAI and Anthropic protocols.
//! Providers are configured via API key, base URL, and protocol type.

mod generic;

pub use generic::{GenericLLMProvider, LLMProtocol, ModelPricing, ProviderConfig};
