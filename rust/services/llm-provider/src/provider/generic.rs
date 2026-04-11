//! Generic LLM Provider
//!
//! A flexible provider that supports OpenAI and Anthropic protocols.
//! Configure with API-KEY, BASE-URL, PROTOCOL, and MODEL-LIST to connect to various LLM services.

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::HeaderName;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::llm_trait::{CompletionStream, LLMError, LLMProvider, LLMResult, TokenCount};
use crate::types::*;

/// LLM Protocol type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LLMProtocol {
    #[default]
    OpenAI,
    Anthropic,
}

/// Model pricing information (per 1M tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Input cost per 1M tokens
    pub input: f64,
    /// Output cost per 1M tokens
    pub output: f64,
}

impl Default for ModelPricing {
    fn default() -> Self {
        Self {
            input: 0.0,
            output: 0.0,
        }
    }
}

impl ModelPricing {
    /// Create new model pricing
    pub fn new(input: f64, output: f64) -> Self {
        Self { input, output }
    }
}

/// Default pricing for common models (per 1M tokens)
/// Used when user hasn't configured model-specific pricing
fn default_pricing_for_model(model: &str) -> ModelPricing {
    match model {
        // OpenAI models
        "gpt-4o" => ModelPricing::new(2.5, 10.0),
        "gpt-4o-mini" => ModelPricing::new(0.15, 0.6),
        "gpt-4-turbo" => ModelPricing::new(10.0, 30.0),
        "gpt-3.5-turbo" => ModelPricing::new(0.5, 1.5),
        // Anthropic models
        "claude-opus-4-6" | "claude-sonnet-4-6" => ModelPricing::new(3.0, 15.0),
        "claude-haiku-4-5-20251001" | "claude-haiku" => ModelPricing::new(0.25, 1.25),
        "claude-3-5-sonnet" | "claude-3-5-sonnet-20241022" => ModelPricing::new(3.0, 15.0),
        "claude-3-opus" => ModelPricing::new(15.0, 75.0),
        "claude-3-sonnet" => ModelPricing::new(3.0, 15.0),
        "claude-3-haiku" => ModelPricing::new(0.25, 1.25),
        // Default - no charge
        _ => ModelPricing::default(),
    }
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (e.g., "anthropic", "openai", "custom")
    pub name: String,
    /// API key for authentication
    pub api_key: String,
    /// Base URL of the API endpoint
    pub base_url: String,
    /// Protocol type (openai or anthropic)
    #[serde(rename = "type")]
    pub protocol: LLMProtocol,
    /// List of supported models
    pub models: Vec<String>,
    /// Default model
    #[serde(default)]
    pub default_model: Option<String>,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Per-model pricing (model_name -> Pricing)
    /// If a model is not listed here, default pricing will be used
    #[serde(default)]
    pub model_pricing: HashMap<String, ModelPricing>,
}

fn default_timeout() -> u64 {
    600 // 10 minutes for streaming responses
}

impl ProviderConfig {
    /// Get the default model
    pub fn default_model(&self) -> &str {
        self.default_model
            .as_deref()
            .or(self.models.first().map(|s| s.as_str()))
            .unwrap_or("gpt-4o")
    }
}

/// Generic LLM Provider - supports OpenAI and Anthropic protocols
pub struct GenericLLMProvider {
    config: ProviderConfig,
    client: Client,
}

impl GenericLLMProvider {
    /// Create a new provider with configuration
    pub fn new(config: ProviderConfig) -> LLMResult<Self> {
        // Disable gzip and brotli decompression to avoid decoding issues
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .no_gzip() // Disable automatic gzip decompression
            .no_brotli() // Disable automatic brotli decompression
            .build()
            .map_err(|e| {
                LLMError::InferenceFailed(format!("failed to create HTTP client: {}", e))
            })?;

        Ok(Self { config, client })
    }

    /// Create from environment variables
    ///
    /// Reads from environment variables:
    /// - `LLM_API_KEY`
    /// - `LLM_BASE_URL`
    /// - `LLM_PROTOCOL` (openai or anthropic)
    /// - `LLM_MODELS` (comma-separated list)
    /// - `LLM_DEFAULT_MODEL`
    pub fn from_env() -> LLMResult<Self> {
        let api_key = std::env::var("LLM_API_KEY")
            .map_err(|_| LLMError::InvalidRequest("LLM_API_KEY not set".to_string()))?;

        let base_url = std::env::var("LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let protocol = std::env::var("LLM_PROTOCOL").unwrap_or_else(|_| "openai".to_string());

        let protocol = match protocol.to_lowercase().as_str() {
            "anthropic" => LLMProtocol::Anthropic,
            _ => LLMProtocol::OpenAI,
        };

        let models = std::env::var("LLM_MODELS")
            .map(|m| m.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| vec!["gpt-4o".to_string()]);

        let default_model = std::env::var("LLM_DEFAULT_MODEL").ok();

        let config = ProviderConfig {
            name: "env".to_string(),
            api_key,
            base_url,
            protocol,
            models,
            default_model,
            timeout_secs: 120,
            model_pricing: HashMap::new(),
        };

        Self::new(config)
    }

    /// Get provider configuration
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Build the request body based on protocol
    fn build_request_body(&self, request: &ChatCompletionRequest) -> serde_json::Value {
        match self.config.protocol {
            LLMProtocol::OpenAI => self.build_openai_request(request),
            LLMProtocol::Anthropic => self.build_anthropic_request(request),
        }
    }

    /// Build OpenAI-compatible request body
    fn build_openai_request(&self, request: &ChatCompletionRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|msg| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "role".to_string(),
                    serde_json::json!(match msg.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "tool",
                    }),
                );

                obj.insert(
                    "content".to_string(),
                    match &msg.content {
                        Some(Content::Text(text)) => serde_json::json!(text),
                        Some(Content::Blocks(blocks)) => {
                            let content: Vec<serde_json::Value> = blocks
                                .iter()
                                .map(|b| match b {
                                    ContentBlock::Text { text } => {
                                        serde_json::json!({"type": "text", "text": text})
                                    }
                                    ContentBlock::Image { image_url } => {
                                        serde_json::json!({
                                            "type": "image_url",
                                            "image_url": { "url": &image_url.url }
                                        })
                                    }
                                })
                                .collect();
                            serde_json::json!(content)
                        }
                        None => serde_json::json!(""),
                    },
                );

                if let Some(ref tool_calls) = msg.tool_calls {
                    let tc: Vec<serde_json::Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": tc.tool_type,
                                "function": {
                                    "name": tc.function.name,
                                    "arguments": tc.function.arguments
                                }
                            })
                        })
                        .collect();
                    obj.insert("tool_calls".to_string(), serde_json::json!(tc));
                }

                if let Some(ref tool_call_id) = msg.tool_call_id {
                    obj.insert("tool_call_id".to_string(), serde_json::json!(tool_call_id));
                }

                serde_json::Value::Object(obj)
            })
            .collect();

        let mut body = serde_json::Map::new();
        let model = if !request.model.is_empty() {
            &request.model
        } else {
            self.config.default_model()
        };
        body.insert("model".to_string(), serde_json::json!(model));
        body.insert("messages".to_string(), serde_json::json!(messages));

        if request.temperature != 0.7 {
            body.insert(
                "temperature".to_string(),
                serde_json::json!(request.temperature),
            );
        }
        if request.max_tokens != 4096 {
            body.insert(
                "max_tokens".to_string(),
                serde_json::json!(request.max_tokens),
            );
        }
        if request.top_p != 1.0 {
            body.insert("top_p".to_string(), serde_json::json!(request.top_p));
        }
        if let Some(ref stop) = request.stop {
            body.insert("stop".to_string(), serde_json::json!(stop));
        }
        if let Some(ref tools) = request.tools {
            let openai_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.function.name,
                            "description": t.function.description,
                            "parameters": t.function.parameters
                        }
                    })
                })
                .collect();
            body.insert("tools".to_string(), serde_json::json!(openai_tools));
        }
        if request.stream {
            body.insert("stream".to_string(), serde_json::json!(true));
        }

        serde_json::Value::Object(body)
    }

    /// Build Anthropic request body
    fn build_anthropic_request(&self, request: &ChatCompletionRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|msg| {
                serde_json::json!({
                    "role": match msg.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "user",
                    },
                    "content": match &msg.content {
                        Some(Content::Text(text)) => serde_json::json!(text),
                        Some(Content::Blocks(blocks)) => {
                            let content: Vec<serde_json::Value> = blocks
                                .iter()
                                .map(|b| match b {
                                    ContentBlock::Text { text } => {
                                        serde_json::json!({"type": "text", "text": text})
                                    }
                                    ContentBlock::Image { image_url } => {
                                        serde_json::json!({
                                            "type": "image",
                                            "source": {
                                                "type": "url",
                                                "url": &image_url.url
                                            }
                                        })
                                    }
                                })
                                .collect();
                            serde_json::json!(content)
                        }
                        None => serde_json::json!(""),
                    }
                })
            })
            .collect();

        let system_prompt = request
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::System)
            .filter_map(|m| m.content.as_ref())
            .filter_map(|c| {
                if let Content::Text(t) = c {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let model = if !request.model.is_empty() {
            &request.model
        } else {
            self.config.default_model()
        };

        let mut body = serde_json::Map::new();
        body.insert("model".to_string(), serde_json::json!(model));
        body.insert("messages".to_string(), serde_json::json!(messages));
        body.insert(
            "max_tokens".to_string(),
            serde_json::json!(request.max_tokens),
        );

        if request.temperature != 0.7 {
            body.insert(
                "temperature".to_string(),
                serde_json::json!(request.temperature),
            );
        }
        if !system_prompt.is_empty() {
            body.insert("system".to_string(), serde_json::json!(system_prompt));
        }
        if let Some(ref stop) = request.stop {
            body.insert("stop_sequences".to_string(), serde_json::json!(stop));
        }
        if request.stream {
            body.insert("stream".to_string(), serde_json::json!(true));
        }

        serde_json::Value::Object(body)
    }

    /// Get the completions URL based on protocol
    fn completions_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        match self.config.protocol {
            LLMProtocol::OpenAI => format!("{}/chat/completions", base),
            LLMProtocol::Anthropic => format!("{}/messages", base),
        }
    }

    /// Get auth header name and value based on protocol
    fn auth_header(&self) -> (HeaderName, String) {
        match self.config.protocol {
            LLMProtocol::Anthropic => (
                HeaderName::from_static("x-api-key"),
                self.config.api_key.clone(),
            ),
            LLMProtocol::OpenAI => (
                HeaderName::from_static("authorization"),
                format!("Bearer {}", self.config.api_key),
            ),
        }
    }

    /// Parse response based on protocol
    fn parse_response(&self, response: serde_json::Value) -> LLMResult<ChatCompletionResponse> {
        match self.config.protocol {
            LLMProtocol::OpenAI => self.parse_openai_response(response),
            LLMProtocol::Anthropic => self.parse_anthropic_response(response),
        }
    }

    /// Parse OpenAI response
    fn parse_openai_response(
        &self,
        response: serde_json::Value,
    ) -> LLMResult<ChatCompletionResponse> {
        let id = response
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let model = response
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let choices: Vec<Choice> = response
            .get("choices")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        let index = c.get("index")?.as_u64()? as u32;
                        let message = c.get("message")?;

                        let role = match message.get("role")?.as_str()? {
                            "system" => MessageRole::System,
                            "user" => MessageRole::User,
                            "assistant" => MessageRole::Assistant,
                            "tool" => MessageRole::Tool,
                            _ => return None,
                        };

                        let content = message
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(|s| Content::Text(s.to_string()));

                        let finish_reason = c
                            .get("finish_reason")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        Some(Choice {
                            index,
                            message: Message {
                                role,
                                content,
                                tool_calls: None,
                                tool_call_id: None,
                            },
                            finish_reason,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = response
            .get("usage")
            .map(|u| Usage {
                input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        let content = choices
            .first()
            .and_then(|c| c.message.content.clone())
            .and_then(|c| {
                if let Content::Text(t) = c {
                    Some(t)
                } else {
                    None
                }
            });
        let stop_reason = choices.first().and_then(|c| c.finish_reason.clone());

        Ok(ChatCompletionResponse {
            id,
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content,
            model,
            choices,
            usage,
            stop_reason,
        })
    }

    /// Parse Anthropic response
    fn parse_anthropic_response(
        &self,
        response: serde_json::Value,
    ) -> LLMResult<ChatCompletionResponse> {
        let id = response
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let model = response
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // MiniMax returns content as array with potentially multiple blocks:
        // [{"type": "thinking", "thinking": "..."}, {"type": "text", "text": "..."}]
        // We need to find the element with type="text" to get the actual response
        let content = response
            .get("content")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter().find(|c| {
                    c.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "text")
                        .unwrap_or(false)
                })
            })
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        let stop_reason = response
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let usage = response
            .get("usage")
            .map(|u| Usage {
                input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: u
                    .get("input_tokens")
                    .and_then(|i| {
                        u.get("output_tokens")
                            .map(|o| i.as_u64().unwrap_or(0) + o.as_u64().unwrap_or(0))
                    })
                    .unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        let choices = if content.is_some() {
            vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: content.clone().map(Content::Text),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: stop_reason.clone(),
            }]
        } else {
            vec![]
        };

        Ok(ChatCompletionResponse {
            id,
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content,
            model,
            choices,
            usage,
            stop_reason,
        })
    }
}

#[async_trait]
impl LLMProvider for GenericLLMProvider {
    fn new() -> LLMResult<Self>
    where
        Self: Sized,
    {
        Self::from_env()
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn is_initialized(&self) -> bool {
        true
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<ChatCompletionResponse> {
        let model = request.model.clone();
        let url = self.completions_url();
        info!("LLM Request URL: {}", url);
        let body = self.build_request_body(&request);
        info!("LLM Request body: {}", body);
        let (auth_header, auth_value) = self.auth_header();

        // Log request messages
        for (i, msg) in request.messages.iter().enumerate() {
            let role = format!("{:?}", msg.role);
            let content_str = msg
                .content
                .as_ref()
                .map(|c| match c {
                    crate::types::Content::Text(s) => s.clone(),
                    crate::types::Content::Blocks(blocks) => format!("[blocks: {}]", blocks.len()),
                })
                .unwrap_or_default();
            debug!(
                "LLM Request [{}] message[{}]: role={}, content=\"{}\"",
                model, i, role, content_str
            );
        }

        let start = Instant::now();

        let mut req_builder = self.client.post(&url);
        req_builder = req_builder.header("content-type", "application/json");

        // Add protocol-specific headers
        match self.config.protocol {
            LLMProtocol::Anthropic => {
                req_builder = req_builder.header("anthropic-version", "2023-06-01");
            }
            LLMProtocol::OpenAI => {}
        }

        req_builder = req_builder.header(auth_header, &auth_value);

        let response = req_builder.json(&body).send().await.map_err(|e| {
            warn!("LLM API request failed: {}", e);
            if e.is_timeout() {
                LLMError::Timeout
            } else if e.is_connect() {
                LLMError::InferenceFailed(format!("connection failed: {}", e))
            } else {
                LLMError::InferenceFailed(format!("request failed: {}", e))
            }
        })?;

        let latency_ms = start.elapsed().as_millis() as u64;
        info!("LLM API response in {}ms", latency_ms);

        let status = response.status();
        if status.as_u16() == 401 {
            return Err(LLMError::ApiKeyInvalid);
        } else if status.as_u16() == 429 {
            return Err(LLMError::RateLimitExceeded);
        } else if status.as_u16() == 400 {
            let error_body = response.text().await.unwrap_or_default();
            return Err(LLMError::InvalidRequest(error_body));
        } else if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            warn!(
                "LLM API error response ({}): {}",
                status.as_u16(),
                error_body
            );
            return Err(LLMError::InferenceFailed(format!(
                "API returned error: {} - {}",
                status.as_u16(),
                error_body
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LLMError::InferenceFailed(format!("failed to parse response: {}", e)))?;

        let result = self.parse_response(data)?;

        // Log response content or warn if empty
        if let Some(content) = &result.content {
            debug!("LLM Response [{}]: content=\"{}\"", model, content);
        } else {
            warn!(
                "LLM Response [{}]: content extraction failed, choices count: {}",
                model,
                result.choices.len()
            );
            // Log the raw response structure for debugging
            debug!(
                "LLM Response raw: id={}, content={:?}, choices.len={}",
                result.id,
                result.content,
                result.choices.len()
            );
        }

        Ok(result)
    }

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<CompletionStream> {
        let model = request.model.clone();

        // Log streaming request messages
        for (i, msg) in request.messages.iter().enumerate() {
            let role = format!("{:?}", msg.role);
            let content_str = msg
                .content
                .as_ref()
                .map(|c| match c {
                    crate::types::Content::Text(s) => s.clone(),
                    crate::types::Content::Blocks(blocks) => format!("[blocks: {}]", blocks.len()),
                })
                .unwrap_or_default();
            debug!(
                "LLM Stream Request [{}] message[{}]: role={}, content=\"{}\"",
                model, i, role, content_str
            );
        }

        let url = self.completions_url();
        let body = self.build_request_body(&request);
        let (auth_header, auth_value) = self.auth_header();

        info!("LLM streaming request starting for model: {}", model);
        let stream_start = Instant::now();

        let mut req_builder = self.client.post(&url);
        req_builder = req_builder
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .header("accept-encoding", "identity"); // Disable compression to avoid decoding issues

        match self.config.protocol {
            LLMProtocol::Anthropic => {
                req_builder = req_builder.header("anthropic-version", "2023-06-01");
            }
            LLMProtocol::OpenAI => {}
        }

        req_builder = req_builder.header(auth_header, &auth_value);

        let response = req_builder.json(&body).send().await.map_err(|e| {
            warn!("LLM streaming request failed: {}", e);
            LLMError::InferenceFailed(format!("stream request failed: {}", e))
        })?;

        let request_latency_ms = stream_start.elapsed().as_millis() as u64;
        info!(
            "LLM streaming request sent: {}ms to first byte",
            request_latency_ms
        );

        if response.status() == 401 {
            return Err(LLMError::ApiKeyInvalid);
        } else if response.status() == 429 {
            return Err(LLMError::RateLimitExceeded);
        } else if !response.status().is_success() {
            return Err(LLMError::InferenceFailed(format!(
                "stream returned error: {}",
                response.status().as_u16()
            )));
        }

        // Create streaming response using chunks - this is more reliable than bytes_stream
        // because chunk() returns raw HTTP chunks without additional decoding
        let mut response = response;
        let protocol = self.config.protocol;

        // Track first token latency
        let first_token_logged = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let chunk_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

        // Create a stream that yields parsed chunks
        let chunk_stream = async_stream::try_stream! {
            let mut buffer = Vec::new();
            let mut first_byte = true;
            const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024; // 10MB limit for safety

            loop {
                debug!("LLM calling response.chunk()...");
                let chunk = response.chunk().await
                    .map_err(|e| {
                        error!("LLM response.chunk() error: {}, type: {:?}", e, std::any::type_name::<reqwest::Error>());
                        // Try to provide more context about the error
                        if e.is_timeout() {
                            error!("LLM stream timeout after {}ms", stream_start.elapsed().as_millis());
                        } else if e.is_connect() {
                            error!("LLM connection error");
                        } else if e.is_body() {
                            error!("LLM body error - possibly encoding or compression issue");
                        }
                        LLMError::InferenceFailed(format!("chunk error: {}", e))
                    })?;

                let bytes = match chunk {
                    Some(c) => {
                        let b = Bytes::from(c);
                        debug!("LLM received {} bytes from chunk()", b.len());
                        b
                    }
                    None => {
                        info!("LLM response.chunk() returned None - stream ended");
                        break;
                    }
                };

                // Log first byte latency
                if first_byte {
                    let first_byte_latency = stream_start.elapsed().as_millis() as u64;
                    info!("LLM first byte received: {}ms", first_byte_latency);
                    first_byte = false;
                }

                // Process bytes line by line
                buffer.extend_from_slice(&bytes);
                debug!("LLM buffer size after extend: {} bytes", buffer.len());

                // Safety check: prevent unbounded buffer growth
                if buffer.len() > MAX_BUFFER_SIZE {
                    warn!("LLM stream buffer exceeded maximum size ({} bytes), clearing", MAX_BUFFER_SIZE);
                    buffer.clear();
                    continue;
                }

                // Process complete lines
                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    // Safety check: ensure position is valid
                    if newline_pos >= buffer.len() {
                        warn!("Invalid newline position {}, clearing buffer", newline_pos);
                        buffer.clear();
                        break;
                    }

                    let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<_>>();
                    // After draining the line, the buffer now contains remaining bytes
                    // No need for additional drain

                    // Parse line (excluding newline) - handle invalid UTF-8 gracefully
                    let line = std::str::from_utf8(&line_bytes)
                        .unwrap_or("")
                        .trim_end_matches('\n')
                        .trim_end_matches('\r');

                    debug!("LLM processing line: {} chars, starts with 'data:': {}", line.len(), line.starts_with("data:"));

                    // Parse SSE line - skip invalid chunks
                    if let Some(chunk) = Self::parse_stream_chunk_static(&protocol, line) {
                        let count = chunk_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

                        // Log first token latency
                        if !first_token_logged.load(std::sync::atomic::Ordering::Relaxed) {
                            if let Some(ref content) = chunk.content {
                                if !content.is_empty() {
                                    first_token_logged.store(true, std::sync::atomic::Ordering::Relaxed);
                                    let latency = stream_start.elapsed().as_millis() as u64;
                                    info!("LLM first token (chunk {}): {}ms from request start, {} chars",
                                          count, latency, content.len());
                                }
                            }
                        }

                        debug!("LLM stream chunk {}: {} chars", count,
                               chunk.content.as_ref().map_or(0, |s| s.len()));

                        yield chunk;
                    }
                }
                debug!("LLM finished processing lines, remaining buffer: {} bytes", buffer.len());
            }
            info!("LLM chunk loop ended");

            info!("LLM streaming complete: {} chunks, {}ms total",
                  chunk_count.load(std::sync::atomic::Ordering::Relaxed),
                  stream_start.elapsed().as_millis());

            if !first_token_logged.load(std::sync::atomic::Ordering::Relaxed) {
                warn!("LLM no content chunks found in stream");
            }
        };

        Ok(Box::pin(chunk_stream))
    }

    async fn count_tokens(&self, text: &str, _model: &str) -> LLMResult<TokenCount> {
        // Simple estimation: ~4 characters per token
        let count = text.len() / 4;
        Ok(TokenCount { count })
    }

    async fn estimate_cost(&self, request: &ChatCompletionRequest) -> LLMResult<CostEstimate> {
        // Get pricing: user config > default pricing for known models > zero
        let model_pricing = self
            .config
            .model_pricing
            .get(&request.model)
            .cloned()
            .unwrap_or_else(|| default_pricing_for_model(&request.model));

        // Estimate input tokens (simple estimation: ~4 characters per token)
        let input_tokens = request
            .messages
            .iter()
            .map(|m| {
                let content_len = match &m.content {
                    Some(Content::Text(t)) => t.len(),
                    Some(Content::Blocks(blocks)) => blocks
                        .iter()
                        .map(|b| match b {
                            ContentBlock::Text { text } => text.len(),
                            ContentBlock::Image { .. } => 100, // Approximate image token cost
                        })
                        .sum(),
                    None => 0,
                };
                content_len / 4
            })
            .sum::<usize>() as u32;

        // Estimate output tokens: use typical ratio (30%) of max_tokens
        // This is more realistic than using max_tokens directly
        let output_tokens = ((request.max_tokens as f64) * 0.3) as u32;

        // Calculate costs (per 1M tokens)
        let input_cost = (input_tokens as f64 / 1_000_000.0) * model_pricing.input;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * model_pricing.output;
        let total_cost = input_cost + output_cost;

        Ok(CostEstimate {
            input_cost,
            output_cost,
            total_cost,
            currency: "USD".to_string(),
        })
    }

    async fn calculate_cost(&self, usage: &Usage, model: &str) -> LLMResult<CostEstimate> {
        // Get pricing: user config > default pricing for known models > zero
        let model_pricing = self
            .config
            .model_pricing
            .get(model)
            .cloned()
            .unwrap_or_else(|| default_pricing_for_model(model));

        // Calculate actual costs from usage (per 1M tokens)
        let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * model_pricing.input;
        let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * model_pricing.output;
        let total_cost = input_cost + output_cost;

        Ok(CostEstimate {
            input_cost,
            output_cost,
            total_cost,
            currency: "USD".to_string(),
        })
    }

    async fn list_models(&self) -> LLMResult<Vec<String>> {
        Ok(self.config.models.clone())
    }

    async fn get_model_info(&self, model: &str) -> LLMResult<ModelInfo> {
        if self.config.models.contains(&model.to_string()) {
            // Get pricing: user config > default pricing for known models > zero
            let model_pricing = self
                .config
                .model_pricing
                .get(model)
                .cloned()
                .unwrap_or_else(|| default_pricing_for_model(model));

            Ok(ModelInfo {
                id: model.to_string(),
                name: model.to_string(),
                provider: self.config.name.clone(),
                context_length: 200_000,
                pricing: Pricing {
                    input: model_pricing.input,
                    output: model_pricing.output,
                    currency: "USD".to_string(),
                },
                capabilities: vec!["chat".to_string(), "tools".to_string()],
            })
        } else {
            Err(LLMError::ModelNotFound(model.to_string()))
        }
    }

    async fn health_check(&self) -> LLMResult<ProviderStatus> {
        let start = Instant::now();
        let url = self.completions_url();
        let (auth_header, auth_value) = self.auth_header();

        let response = self
            .client
            .get(&url)
            .header(auth_header, &auth_value)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match response {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 405 => {
                Ok(ProviderStatus {
                    name: self.config.name.clone(),
                    healthy: true,
                    latency_ms,
                    error_rate: 0.0,
                    last_check: chrono::Utc::now().to_rfc3339(),
                })
            }
            Ok(_) => Ok(ProviderStatus {
                name: self.config.name.clone(),
                healthy: false,
                latency_ms,
                error_rate: 1.0,
                last_check: chrono::Utc::now().to_rfc3339(),
            }),
            Err(_) => Ok(ProviderStatus {
                name: self.config.name.clone(),
                healthy: false,
                latency_ms,
                error_rate: 1.0,
                last_check: chrono::Utc::now().to_rfc3339(),
            }),
        }
    }
}

impl GenericLLMProvider {
    /// Parse streaming chunk (simplified)
    fn parse_stream_chunk(&self, line: &str) -> Option<ChatCompletionChunk> {
        Self::parse_stream_chunk_static(&self.config.protocol, line)
    }

    /// Static version of parse_stream_chunk that can be used in async streams
    fn parse_stream_chunk_static(
        protocol: &LLMProtocol,
        line: &str,
    ) -> Option<ChatCompletionChunk> {
        if !line.starts_with("data: ") || line == "data: [DONE]" {
            return None;
        }

        let json_str = &line[6..];
        let data: serde_json::Value = serde_json::from_str(json_str).ok()?;

        debug!(
            "LLM stream chunk parsed: id={}",
            data.get("id").and_then(|v| v.as_str()).unwrap_or("")
        );

        let id = data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let model = data
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Handle both OpenAI and Anthropic streaming formats
        let content = match protocol {
            LLMProtocol::OpenAI => data
                .get("choices")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string()),
            LLMProtocol::Anthropic => {
                // MiniMax streaming format uses delta.type to indicate "text_delta" or "thinking_delta"
                // Also handles non-streaming format where content is an array of blocks
                data.get("delta")
                    .and_then(|d| {
                        let delta_type = d.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        match delta_type {
                            "text_delta" => d
                                .get("text")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()),
                            "thinking_delta" => d
                                .get("thinking")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()),
                            _ => None,
                        }
                    })
                    // Also check for non-streaming format (content array with text blocks)
                    .or_else(|| {
                        data.get("content")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| {
                                arr.iter().find(|c| {
                                    c.get("type")
                                        .and_then(|t| t.as_str())
                                        .map(|t| t == "text")
                                        .unwrap_or(false)
                                })
                            })
                            .and_then(|c| c.get("text"))
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
            }
        };

        // Track if this chunk contains thinking content (for MiniMax Anthropic format)
        let is_thinking = match protocol {
            LLMProtocol::Anthropic => data
                .get("delta")
                .and_then(|d| d.get("type"))
                .and_then(|t| t.as_str())
                .map(|t| t == "thinking_delta"),
            _ => Some(false),
        };

        Some(ChatCompletionChunk {
            id,
            type_field: "chat.completion.chunk".to_string(),
            role: None,
            content,
            is_thinking,
            model,
            choices: vec![],
        })
    }
}

impl Clone for GenericLLMProvider {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: self.client.clone(),
        }
    }
}
