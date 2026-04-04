//! Anthropic Provider Implementation
//!
//! Implementation for Anthropic Claude API.

use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::llm_trait::{CompletionStream, LLMError, LLMProvider, LLMResult, TokenCount};
use crate::types::*;

/// Anthropic API base URL
const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";

/// Anthropic provider implementation
pub struct AnthropicProvider {
    name: String,
    api_key: String,
    client: Client,
    initialized: bool,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(api_key: impl Into<String>) -> LLMResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| LLMError::InferenceFailed(format!("failed to create HTTP client: {}", e)))?;

        Ok(Self {
            name: "anthropic".to_string(),
            api_key: api_key.into(),
            client,
            initialized: true,
            base_url: ANTHROPIC_BASE_URL.to_string(),
        })
    }

    /// Create with custom base URL (for testing or custom endpoints)
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> LLMResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| LLMError::InferenceFailed(format!("failed to create HTTP client: {}", e)))?;

        Ok(Self {
            name: "anthropic".to_string(),
            api_key: api_key.into(),
            client,
            initialized: true,
            base_url: base_url.into(),
        })
    }

    /// Build the request body for Anthropic API
    fn build_request_body(&self, request: &ChatCompletionRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|msg| {
                let mut obj = serde_json::Map::new();
                obj.insert("role".to_string(), serde_json::json!(match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "user",
                }));
                obj.insert("content".to_string(), match &msg.content {
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
                });
                serde_json::Value::Object(obj)
            })
            .collect();

        let mut body = serde_json::Map::new();
        body.insert("model".to_string(), serde_json::json!(&request.model));
        body.insert("messages".to_string(), serde_json::json!(messages));
        body.insert("max_tokens".to_string(), serde_json::json!(request.max_tokens));

        if request.temperature != 0.7 {
            body.insert("temperature".to_string(), serde_json::json!(request.temperature));
        }

        if request.top_p != 1.0 {
            body.insert("top_p".to_string(), serde_json::json!(request.top_p));
        }

        if let Some(ref stop) = request.stop {
            body.insert("stop_sequences".to_string(), serde_json::json!(stop));
        }

        if let Some(ref tools) = request.tools {
            let anthropic_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.function.name,
                        "description": t.function.description,
                        "input_schema": t.function.parameters
                    })
                })
                .collect();
            body.insert("tools".to_string(), serde_json::json!(anthropic_tools));
        }

        serde_json::Value::Object(body)
    }

    /// Parse Anthropic response to ChatCompletionResponse
    fn parse_response(&self, response: serde_json::Value) -> LLMResult<ChatCompletionResponse> {
        let id = response
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let model = response
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.name)
            .to_string();

        let content = response.get("content").and_then(|v| v.as_array());

        let message_content = content
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        let stop_reason = response
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let usage = response.get("usage").map(|u| Usage {
            input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: u
                .get("input_tokens")
                .map(|i| i.as_u64().unwrap_or(0) + u.get("output_tokens").and_then(|o| o.as_u64()).unwrap_or(0))
                .unwrap_or(0) as u32,
        }).unwrap_or_default();

        let choices = if let Some(_content) = content {
            vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: message_content.clone().map(Content::Text),
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
            content: message_content,
            model,
            choices,
            usage,
            stop_reason,
        })
    }

    /// Parse streaming chunk from SSE data
    fn parse_stream_chunk(&self, line: &str) -> Option<ChatCompletionChunk> {
        if !line.starts_with("data: ") {
            return None;
        }

        let json_str = &line[6..];
        if json_str == "[DONE]" {
            return None;
        }

        let data: serde_json::Value = serde_json::from_str(json_str).ok()?;

        let chunk_type = data.get("type")?.as_str()?;

        match chunk_type {
            "content_block_delta" => {
                let delta = data.get("delta")?;
                let index = data.get("index")?.as_u64()? as u32;

                let text = delta.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());
                let text_clone = text.clone();

                Some(ChatCompletionChunk {
                    id: data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    type_field: chunk_type.to_string(),
                    role: None,
                    content: text,
                    model: data.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    choices: vec![ChoiceChunk {
                        index,
                        delta: Delta::ContentBlockDelta {
                            delta: DeltaContent { text: text_clone, name: None },
                            index,
                        },
                        finish_reason: None,
                    }],
                })
            }
            "message_delta" => {
                let delta = data.get("delta")?;
                let index = data.get("index")?.as_u64()? as u32;
                let content = delta.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());
                let finish_reason = delta.get("stop_reason").and_then(|v| v.as_str()).map(|s| s.to_string());

                Some(ChatCompletionChunk {
                    id: data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    type_field: chunk_type.to_string(),
                    role: None,
                    content,
                    model: data.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    choices: vec![ChoiceChunk {
                        index,
                        delta: Delta::MessageDelta {
                            delta: MessageDeltaContent {
                                role: None,
                                content: None,
                                tool_calls: None,
                            },
                            index,
                        },
                        finish_reason,
                    }],
                })
            }
            "message_stop" => Some(ChatCompletionChunk {
                id: data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                type_field: chunk_type.to_string(),
                role: None,
                content: None,
                model: "".to_string(),
                choices: vec![],
            }),
            _ => None,
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn new() -> LLMResult<Self>
    where
        Self: Sized,
    {
        Err(LLMError::NotInitialized)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<ChatCompletionResponse> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_request_body(&request);

        let start = Instant::now();

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                warn!("Anthropic API request failed: {}", e);
                if e.is_timeout() {
                    LLMError::Timeout
                } else if e.is_connect() {
                    LLMError::InferenceFailed(format!("connection failed: {}", e))
                } else {
                    LLMError::InferenceFailed(format!("request failed: {}", e))
                }
            })?;

        let latency_ms = start.elapsed().as_millis() as u64;
        info!("Anthropic API response in {}ms", latency_ms);

        let status = response.status();
        if status.as_u16() == 401 {
            return Err(LLMError::ApiKeyInvalid);
        } else if status.as_u16() == 429 {
            return Err(LLMError::RateLimitExceeded);
        } else if status.as_u16() == 400 {
            let error_body = response.text().await.unwrap_or_default();
            return Err(LLMError::InvalidRequest(error_body));
        } else if !status.is_success() {
            return Err(LLMError::InferenceFailed(format!(
                "API returned error: {}",
                status.as_u16()
            )));
        }

        let data: serde_json::Value = response.json().await.map_err(|e| {
            LLMError::InferenceFailed(format!("failed to parse response: {}", e))
        })?;

        self.parse_response(data)
    }

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<CompletionStream> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_request_body(&request);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                warn!("Anthropic streaming request failed: {}", e);
                LLMError::InferenceFailed(format!("stream request failed: {}", e))
            })?;

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

        // For now, collect streaming response and yield as single chunk
        // TODO: Implement proper SSE streaming
        let body = response.text().await.map_err(|e| {
            LLMError::InferenceFailed(format!("failed to read streaming response: {}", e))
        })?;

        let provider = Arc::new(self.clone());
        let chunks: Vec<LLMResult<ChatCompletionChunk>> = body
            .lines()
            .filter_map(|line| provider.parse_stream_chunk(line).map(Ok))
            .collect();

        Ok(Box::pin(futures::stream::iter(chunks)))
    }

    async fn count_tokens(&self, text: &str, _model: &str) -> LLMResult<TokenCount> {
        // Simple estimation: ~4 characters per token for English
        // For accurate counting, use tiktoken or similar
        let count = text.len() / 4;
        Ok(TokenCount { count })
    }

    async fn estimate_cost(&self, request: &ChatCompletionRequest) -> LLMResult<CostEstimate> {
        let pricing = self.get_model_info(&request.model).await?;
        let input_cost = pricing.pricing.input * request.messages.len() as f64 * 0.001;
        let output_cost = pricing.pricing.output * request.max_tokens as f64 * 0.001;

        Ok(CostEstimate {
            input_cost,
            output_cost,
            total_cost: input_cost + output_cost,
            currency: pricing.pricing.currency,
        })
    }

    async fn list_models(&self) -> LLMResult<Vec<String>> {
        Ok(vec![
            "claude-sonnet-4-6".to_string(),
            "claude-haiku".to_string(),
        ])
    }

    async fn get_model_info(&self, model: &str) -> LLMResult<ModelInfo> {
        match model {
            "claude-sonnet-4-6" => Ok(ModelInfo {
                id: "claude-sonnet-4-6".to_string(),
                name: "Claude Sonnet 4".to_string(),
                provider: "anthropic".to_string(),
                context_length: 200_000,
                pricing: Pricing {
                    input: 3.0,
                    output: 15.0,
                    currency: "USD".to_string(),
                },
                capabilities: vec![
                    "chat".to_string(),
                    "tools".to_string(),
                    "vision".to_string(),
                ],
            }),
            "claude-haiku" => Ok(ModelInfo {
                id: "claude-haiku".to_string(),
                name: "Claude Haiku".to_string(),
                provider: "anthropic".to_string(),
                context_length: 200_000,
                pricing: Pricing {
                    input: 0.25,
                    output: 1.25,
                    currency: "USD".to_string(),
                },
                capabilities: vec!["chat".to_string(), "tools".to_string()],
            }),
            _ => Err(LLMError::ModelNotFound(model.to_string())),
        }
    }

    async fn health_check(&self) -> LLMResult<ProviderStatus> {
        let start = Instant::now();
        let url = format!("{}/v1/models", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match response {
            Ok(resp) if resp.status().is_success() => Ok(ProviderStatus {
                name: self.name.clone(),
                healthy: true,
                latency_ms,
                error_rate: 0.0,
                last_check: chrono::Utc::now().to_rfc3339(),
            }),
            Ok(_resp) => Ok(ProviderStatus {
                name: self.name.clone(),
                healthy: false,
                latency_ms,
                error_rate: 1.0,
                last_check: chrono::Utc::now().to_rfc3339(),
            }),
            Err(_) => Ok(ProviderStatus {
                name: self.name.clone(),
                healthy: false,
                latency_ms,
                error_rate: 1.0,
                last_check: chrono::Utc::now().to_rfc3339(),
            }),
        }
    }
}

impl Clone for AnthropicProvider {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            api_key: self.api_key.clone(),
            client: self.client.clone(),
            initialized: self.initialized,
            base_url: self.base_url.clone(),
        }
    }
}
