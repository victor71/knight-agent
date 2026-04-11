//! LLM Provider Types
//!
//! Core data types for LLM provider abstraction.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Message role for LLM API
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

// Custom serialization to lowercase string for API compatibility
impl Serialize for MessageRole {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        serializer.serialize_str(s)
    }
}

// Custom deserialization to support both lowercase string and tagged format
impl<'de> Deserialize<'de> for MessageRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MessageRoleVisitor;

        impl<'de> serde::de::Visitor<'de> for MessageRoleVisitor {
            type Value = MessageRole;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or an object with type field")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value.to_lowercase().as_str() {
                    "system" => Ok(MessageRole::System),
                    "user" => Ok(MessageRole::User),
                    "assistant" => Ok(MessageRole::Assistant),
                    "tool" => Ok(MessageRole::Tool),
                    _ => Err(serde::de::Error::unknown_variant(value, VARIANTS)),
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // Support old tagged format: {"type": "User"}
                if let Some(key) = map.next_key::<String>()? {
                    if key == "type" {
                        let value = map.next_value::<String>()?;
                        return self.visit_str(&value);
                    }
                }
                Err(serde::de::Error::custom(
                    "expected object with 'type' field",
                ))
            }
        }

        const VARIANTS: &[&str] = &["system", "user", "assistant", "tool"];

        deserializer.deserialize_any(MessageRoleVisitor)
    }
}

/// Content block for multi-modal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    Text { text: String },
    Image { image_url: ImageUrl },
}

/// Image URL for image content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    pub detail: Option<String>,
}

/// Tool call from assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolCallFunction,
}

/// Tool call function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

/// Tool function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Message content (text or content blocks)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub top_p: f32,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(default)]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default)]
    pub stream: bool,
}

impl Default for ChatCompletionRequest {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-6".to_string(),
            messages: Vec::new(),
            temperature: 0.7,
            max_tokens: 4096,
            top_p: 1.0,
            stop: None,
            tools: None,
            tool_choice: None,
            stream: false,
        }
    }
}

fn default_max_tokens() -> u32 {
    4096
}

/// Tool choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto,
    None,
    Function(FunctionChoice),
}

/// Function choice for tool selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChoice {
    #[serde(rename = "type")]
    pub type_field: String,
    pub function: FunctionRef,
}

/// Function reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRef {
    pub name: String,
}

/// Chat completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Chat completion chunk delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Delta {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: DeltaContent, index: u32 },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaContent,
        index: u32,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
}

/// Delta content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Message delta content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Choice chunk for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceChunk {
    pub index: u32,
    pub delta: Delta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Chat completion chunk for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub role: Option<String>,
    pub content: Option<String>,
    /// Whether this chunk contains thinking content (should be filtered by UI)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_thinking: Option<bool>,
    pub model: String,
    pub choices: Vec<ChoiceChunk>,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub role: Option<String>,
    pub content: Option<String>,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
    pub stop_reason: Option<String>,
}

/// Token usage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(rename = "input_tokens")]
    pub input_tokens: u32,
    #[serde(rename = "output_tokens")]
    pub output_tokens: u32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
}

/// Cost estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    #[serde(rename = "input_cost")]
    pub input_cost: f64,
    #[serde(rename = "output_cost")]
    pub output_cost: f64,
    #[serde(rename = "total_cost")]
    pub total_cost: f64,
    pub currency: String,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    #[serde(rename = "context_length")]
    pub context_length: u32,
    pub pricing: Pricing,
    pub capabilities: Vec<String>,
}

/// Pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    pub input: f64,
    pub output: f64,
    pub currency: String,
}

/// Provider type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    OpenAi,
    Custom,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub models: Vec<String>,
    #[serde(default)]
    pub timeout: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_max_retries() -> u32 {
    3
}

/// Provider status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub name: String,
    pub healthy: bool,
    #[serde(rename = "latency_ms")]
    pub latency_ms: u64,
    #[serde(rename = "error_rate")]
    pub error_rate: f64,
    #[serde(rename = "last_check")]
    pub last_check: String,
}

/// Routing strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RoutingStrategy {
    Cost,
    Quality,
    Speed,
    #[default]
    Auto,
}

/// Routed request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedRequest {
    #[serde(rename = "original_request")]
    pub original_request: ChatCompletionRequest,
    #[serde(rename = "routed_model")]
    pub routed_model: String,
    #[serde(rename = "routed_provider")]
    pub routed_provider: String,
    pub reason: String,
}
