//! OpenAI Provider Implementation
//!
//! Implementation for OpenAI Chat API.

use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::llm_trait::{CompletionStream, LLMError, LLMProvider, LLMResult, TokenCount};
use crate::types::*;

/// OpenAI API base URL
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// OpenAI provider implementation
pub struct OpenAIProvider {
    name: String,
    api_key: String,
    client: Client,
    initialized: bool,
    base_url: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: impl Into<String>) -> LLMResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| LLMError::InferenceFailed(format!("failed to create HTTP client: {}", e)))?;

        Ok(Self {
            name: "openai".to_string(),
            api_key: api_key.into(),
            client,
            initialized: true,
            base_url: OPENAI_BASE_URL.to_string(),
        })
    }

    /// Create with custom base URL (for testing or custom endpoints)
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> LLMResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| LLMError::InferenceFailed(format!("failed to create HTTP client: {}", e)))?;

        Ok(Self {
            name: "openai".to_string(),
            api_key: api_key.into(),
            client,
            initialized: true,
            base_url: base_url.into(),
        })
    }

    /// Build the request body for OpenAI API
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
                    MessageRole::Tool => "tool",
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
                                        "type": "image_url",
                                        "image_url": {
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
        body.insert("model".to_string(), serde_json::json!(&request.model));
        body.insert("messages".to_string(), serde_json::json!(messages));

        if request.temperature != 0.7 {
            body.insert("temperature".to_string(), serde_json::json!(request.temperature));
        }

        if request.max_tokens != 4096 {
            body.insert("max_tokens".to_string(), serde_json::json!(request.max_tokens));
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

        if let Some(ref tool_choice) = request.tool_choice {
            match tool_choice {
                ToolChoice::Auto => {
                    body.insert("tool_choice".to_string(), serde_json::json!("auto"));
                }
                ToolChoice::None => {
                    body.insert("tool_choice".to_string(), serde_json::json!("none"));
                }
                ToolChoice::Function(fc) => {
                    body.insert("tool_choice".to_string(), serde_json::json!({
                        "type": "function",
                        "function": {"name": fc.function.name}
                    }));
                }
            }
        }

        if request.stream {
            body.insert("stream".to_string(), serde_json::json!(true));
        }

        serde_json::Value::Object(body)
    }

    /// Parse OpenAI response to ChatCompletionResponse
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

                        let tool_calls = message.get("tool_calls").and_then(|v| v.as_array()).map(
                            |calls| {
                                calls.iter().filter_map(|tc| {
                                    Some(ToolCall {
                                        id: tc.get("id")?.as_str()?.to_string(),
                                        tool_type: tc.get("type")?.as_str()?.to_string(),
                                        function: ToolCallFunction {
                                            name: tc.get("function")?
                                                .get("name")?
                                                .as_str()?
                                                .to_string(),
                                            arguments: tc.get("function")
                                                ?.get("arguments")?
                                                .as_str()?
                                                .to_string(),
                                        },
                                    })
                                }).collect()
                            },
                        );

                        let finish_reason = c
                            .get("finish_reason")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        Some(Choice {
                            index,
                            message: Message {
                                role,
                                content,
                                tool_calls,
                                tool_call_id: None,
                            },
                            finish_reason,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = response.get("usage").map(|u| Usage {
            input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            output_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        }).unwrap_or_default();

        let content = choices
            .first()
            .and_then(|c| c.message.content.clone())
            .and_then(|c| if let Content::Text(t) = c { Some(t) } else { None });

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

        let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let model = data.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let choices: Vec<ChoiceChunk> = data.get("choices").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let index = c.get("index")?.as_u64()? as u32;
                    let delta = c.get("delta")?;

                    let (delta_type, content, tool_calls) = if let Some(tc) = delta.get("tool_calls") {
                        let calls: Option<Vec<ToolCall>> = tc.as_array().map(|arr| {
                            arr.iter().filter_map(|t| {
                                Some(ToolCall {
                                    id: t.get("id")?.as_str()?.to_string(),
                                    tool_type: t.get("type")?.as_str()?.to_string(),
                                    function: ToolCallFunction {
                                        name: t.get("function")?.get("name")?.as_str()?.to_string(),
                                        arguments: t.get("function")?.get("arguments")?.as_str()?.to_string(),
                                    },
                                })
                            }).collect()
                        });
                        ("tool_calls".to_string(), None, calls)
                    } else {
                        let text = delta.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());
                        ("content".to_string(), text, None)
                    };

                    let finish_reason = c.get("finish_reason").and_then(|v| v.as_str()).map(|s| s.to_string());

                    Some(ChoiceChunk {
                        index,
                        delta: match delta_type.as_str() {
                            "tool_calls" => Delta::MessageDelta {
                                delta: MessageDeltaContent {
                                    role: None,
                                    content: None,
                                    tool_calls,
                                },
                                index,
                            },
                            _ => Delta::ContentBlockDelta {
                                delta: DeltaContent { text: content, name: None },
                                index,
                            },
                        },
                        finish_reason,
                    })
                })
                .collect()
        }).unwrap_or_default();

        let content = choices
            .first()
            .and_then(|c| {
                if let Delta::ContentBlockDelta { delta, .. } = &c.delta {
                    delta.text.clone()
                } else {
                    None
                }
            });

        Some(ChatCompletionChunk {
            id,
            type_field: "chat.completion.chunk".to_string(),
            role: None,
            content,
            model,
            choices,
        })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
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
        let url = format!("{}/chat/completions", self.base_url);
        let body = self.build_request_body(&request);

        let start = Instant::now();

        let response = self
            .client
            .post(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                warn!("OpenAI API request failed: {}", e);
                if e.is_timeout() {
                    LLMError::Timeout
                } else if e.is_connect() {
                    LLMError::InferenceFailed(format!("connection failed: {}", e))
                } else {
                    LLMError::InferenceFailed(format!("request failed: {}", e))
                }
            })?;

        let latency_ms = start.elapsed().as_millis() as u64;
        info!("OpenAI API response in {}ms", latency_ms);

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
        let url = format!("{}/chat/completions", self.base_url);
        let body = self.build_request_body(&request);

        let response = self
            .client
            .post(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                warn!("OpenAI streaming request failed: {}", e);
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
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
        ])
    }

    async fn get_model_info(&self, model: &str) -> LLMResult<ModelInfo> {
        match model {
            "gpt-4o" => Ok(ModelInfo {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider: "openai".to_string(),
                context_length: 128_000,
                pricing: Pricing {
                    input: 2.5,
                    output: 10.0,
                    currency: "USD".to_string(),
                },
                capabilities: vec![
                    "chat".to_string(),
                    "tools".to_string(),
                    "vision".to_string(),
                ],
            }),
            "gpt-4o-mini" => Ok(ModelInfo {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o Mini".to_string(),
                provider: "openai".to_string(),
                context_length: 128_000,
                pricing: Pricing {
                    input: 0.15,
                    output: 0.60,
                    currency: "USD".to_string(),
                },
                capabilities: vec!["chat".to_string(), "tools".to_string()],
            }),
            "gpt-4-turbo" => Ok(ModelInfo {
                id: "gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                provider: "openai".to_string(),
                context_length: 128_000,
                pricing: Pricing {
                    input: 10.0,
                    output: 30.0,
                    currency: "USD".to_string(),
                },
                capabilities: vec![
                    "chat".to_string(),
                    "tools".to_string(),
                    "vision".to_string(),
                ],
            }),
            _ => Err(LLMError::ModelNotFound(model.to_string())),
        }
    }

    async fn health_check(&self) -> LLMResult<ProviderStatus> {
        let start = Instant::now();
        let url = format!("{}/models", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
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

impl Clone for OpenAIProvider {
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
