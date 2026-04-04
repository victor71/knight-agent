//! LLM Provider Unit Tests

use std::collections::HashMap;

use llm_provider::{
    provider::{GenericLLMProvider, LLMProtocol, ModelPricing, ProviderConfig},
    ChatCompletionRequest, Content, Message, MessageRole,
    LLMProvider,
};

fn create_openai_provider() -> GenericLLMProvider {
    let mut model_pricing = HashMap::new();
    model_pricing.insert(
        "gpt-4o".to_string(),
        ModelPricing::new(2.5, 10.0), // $2.5/M input, $10/M output
    );
    model_pricing.insert(
        "gpt-4o-mini".to_string(),
        ModelPricing::new(0.15, 0.6), // $0.15/M input, $0.6/M output
    );

    let config = ProviderConfig {
        name: "test-openai".to_string(),
        api_key: "test-api-key".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        protocol: LLMProtocol::OpenAI,
        models: vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
        ],
        default_model: Some("gpt-4o".to_string()),
        timeout_secs: 120,
        model_pricing,
    };
    GenericLLMProvider::new(config).unwrap()
}

fn create_anthropic_provider() -> GenericLLMProvider {
    let mut model_pricing = HashMap::new();
    model_pricing.insert(
        "claude-sonnet-4-6".to_string(),
        ModelPricing::new(3.0, 15.0), // $3/M input, $15/M output
    );
    model_pricing.insert(
        "claude-haiku".to_string(),
        ModelPricing::new(0.25, 1.25), // $0.25/M input, $1.25/M output
    );

    let config = ProviderConfig {
        name: "test-anthropic".to_string(),
        api_key: "test-api-key".to_string(),
        base_url: "https://api.anthropic.com".to_string(),
        protocol: LLMProtocol::Anthropic,
        models: vec![
            "claude-sonnet-4-6".to_string(),
            "claude-haiku".to_string(),
        ],
        default_model: Some("claude-sonnet-4-6".to_string()),
        timeout_secs: 120,
        model_pricing,
    };
    GenericLLMProvider::new(config).unwrap()
}

fn create_test_message() -> Message {
    Message {
        role: MessageRole::User,
        content: Some(Content::Text("Hello!".to_string())),
        tool_calls: None,
        tool_call_id: None,
    }
}

// Provider creation tests

#[test]
fn test_create_openai_provider() {
    let provider = create_openai_provider();
    assert!(provider.is_initialized());
    assert_eq!(provider.name(), "test-openai");
}

#[test]
fn test_create_anthropic_provider() {
    let provider = create_anthropic_provider();
    assert!(provider.is_initialized());
    assert_eq!(provider.name(), "test-anthropic");
}

#[test]
fn test_provider_config_validation() {
    let config = ProviderConfig {
        name: "test".to_string(),
        api_key: "key".to_string(),
        base_url: "https://api.test.com".to_string(),
        protocol: LLMProtocol::OpenAI,
        models: vec!["model1".to_string()],
        default_model: None,
        timeout_secs: 60,
        model_pricing: HashMap::new(),
    };

    // default_model returns first model when default_model is None
    assert_eq!(config.default_model(), "model1");
}

#[test]
fn test_protocol_enum_serialization() {
    // Test OpenAI protocol
    let json = serde_json::to_string(&LLMProtocol::OpenAI).unwrap();
    assert_eq!(json, "\"openai\"");

    let parsed: LLMProtocol = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, LLMProtocol::OpenAI);

    // Test Anthropic protocol
    let json = serde_json::to_string(&LLMProtocol::Anthropic).unwrap();
    assert_eq!(json, "\"anthropic\"");

    let parsed: LLMProtocol = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, LLMProtocol::Anthropic);
}

// Model info tests

#[tokio::test]
async fn test_list_models_openai() {
    let provider = create_openai_provider();
    let models = provider.list_models().await.unwrap();
    assert_eq!(models.len(), 2);
    assert!(models.contains(&"gpt-4o".to_string()));
    assert!(models.contains(&"gpt-4o-mini".to_string()));
}

#[tokio::test]
async fn test_list_models_anthropic() {
    let provider = create_anthropic_provider();
    let models = provider.list_models().await.unwrap();
    assert_eq!(models.len(), 2);
    assert!(models.contains(&"claude-sonnet-4-6".to_string()));
    assert!(models.contains(&"claude-haiku".to_string()));
}

#[tokio::test]
async fn test_get_model_info_existing() {
    let provider = create_openai_provider();
    let info = provider.get_model_info("gpt-4o").await.unwrap();
    assert_eq!(info.id, "gpt-4o");
    assert_eq!(info.name, "gpt-4o");
    assert_eq!(info.provider, "test-openai");
    assert!(info.capabilities.contains(&"chat".to_string()));
}

#[tokio::test]
async fn test_get_model_info_nonexistent() {
    let provider = create_openai_provider();
    let result = provider.get_model_info("nonexistent-model").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_model_info_with_pricing() {
    let provider = create_openai_provider();
    let info = provider.get_model_info("gpt-4o").await.unwrap();

    // Check pricing is set from config ($2.5/M input, $10/M output)
    assert_eq!(info.pricing.input, 2.5);
    assert_eq!(info.pricing.output, 10.0);
    assert_eq!(info.pricing.currency, "USD");
}

// Token counting tests

#[tokio::test]
async fn test_count_tokens() {
    let provider = create_openai_provider();
    let text = "Hello, this is a test message for token counting.";

    let result = provider.count_tokens(text, "gpt-4o").await.unwrap();
    // Simple estimation: ~4 characters per token
    assert_eq!(result.count, text.len() / 4);
}

// Cost estimation tests

#[tokio::test]
async fn test_estimate_cost() {
    let provider = create_openai_provider();
    let request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![create_test_message()],
        temperature: 0.7,
        max_tokens: 100,
        ..Default::default()
    };

    let cost = provider.estimate_cost(&request).await.unwrap();
    // "Hello!" = 6 chars -> ~1 token (6/4), $2.5/M input
    // max_tokens = 100, $10/M output
    // input_cost = 1/1M * 2.5 = 0.0000025
    // output_cost = 100/1M * 10 = 0.001
    assert!(cost.input_cost > 0.0);
    assert!(cost.output_cost > 0.0);
    assert!(cost.total_cost > 0.0);
    assert_eq!(cost.currency, "USD".to_string());
}

#[tokio::test]
async fn test_calculate_cost_actual_usage() {
    use llm_provider::types::Usage;

    let provider = create_openai_provider();
    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 500,
        total_tokens: 1500,
    };

    let cost = provider.calculate_cost(&usage, "gpt-4o").await.unwrap();
    // 1000 input tokens at $2.5/M = $0.0025
    // 500 output tokens at $10/M = $0.005
    // total = $0.0075
    assert_eq!(cost.input_cost, 0.0025);
    assert_eq!(cost.output_cost, 0.005);
    assert_eq!(cost.total_cost, 0.0075);
    assert_eq!(cost.currency, "USD".to_string());
}

#[tokio::test]
async fn test_calculate_cost_unknown_model() {
    use llm_provider::types::Usage;

    let provider = create_openai_provider();
    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 500,
        total_tokens: 1500,
    };

    // Unknown model should use default (zero) pricing
    let cost = provider.calculate_cost(&usage, "unknown-model").await.unwrap();
    assert_eq!(cost.input_cost, 0.0);
    assert_eq!(cost.output_cost, 0.0);
    assert_eq!(cost.total_cost, 0.0);
}

// Health check tests - skipped because they require network/API access
// #[tokio::test]
// async fn test_health_check_success() {
//     let provider = create_openai_provider();
//     let status = provider.health_check().await.unwrap();
//     assert_eq!(status.name, "test-openai");
// }

// Request building tests

#[test]
fn test_chat_completion_request_default() {
    let request = ChatCompletionRequest::default();
    assert_eq!(request.model, "claude-sonnet-4-6");
    assert!(request.messages.is_empty());
    assert_eq!(request.temperature, 0.7);
    assert_eq!(request.max_tokens, 4096);
    assert_eq!(request.top_p, 1.0);
    assert!(!request.stream);
}

#[test]
fn test_chat_completion_request_with_message() {
    let request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![create_test_message()],
        temperature: 0.5,
        max_tokens: 500,
        ..Default::default()
    };

    assert_eq!(request.model, "gpt-4o");
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.temperature, 0.5);
    assert_eq!(request.max_tokens, 500);
}

// Message serialization tests

#[test]
fn test_message_serialization() {
    let msg = create_test_message();
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.role, MessageRole::User);
    assert!(matches!(parsed.content, Some(Content::Text(_))));
}

#[test]
fn test_message_with_tool_calls() {
    let msg = Message {
        role: MessageRole::Assistant,
        content: Some(Content::Text("Using tool".to_string())),
        tool_calls: Some(vec![]),
        tool_call_id: None,
    };

    let json = serde_json::to_string(&msg).unwrap();
    // MessageRole serializes as the variant name (Assistant), not lowercase
    assert!(json.contains("Assistant"));
    assert!(json.contains("tool_calls"));
}

// Content type tests

#[test]
fn test_content_text() {
    let content = Content::Text("Hello".to_string());
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("Hello"));
}

#[test]
fn test_content_blocks() {
    use llm_provider::types::ContentBlock;

    let content = Content::Blocks(vec![
        ContentBlock::Text {
            text: "Hello".to_string(),
        },
    ]);
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("text"));
    assert!(json.contains("Hello"));
}

// Provider config serialization tests

#[test]
fn test_provider_config_serialization() {
    let config = ProviderConfig {
        name: "test".to_string(),
        api_key: "secret".to_string(),
        base_url: "https://api.test.com".to_string(),
        protocol: LLMProtocol::OpenAI,
        models: vec!["model1".to_string(), "model2".to_string()],
        default_model: Some("model1".to_string()),
        timeout_secs: 60,
        model_pricing: HashMap::new(),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("openai"));
    assert!(json.contains("model1"));

    let parsed: ProviderConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "test");
    assert_eq!(parsed.protocol, LLMProtocol::OpenAI);
    assert_eq!(parsed.models.len(), 2);
}

// Error type tests

#[test]
fn test_llm_error_display() {
    let error = llm_provider::LLMError::NotInitialized;
    assert_eq!(error.to_string(), "provider not initialized");

    let error = llm_provider::LLMError::ModelNotFound("gpt-5".to_string());
    assert_eq!(error.to_string(), "model not found: gpt-5");

    let error = llm_provider::LLMError::InferenceFailed("timeout".to_string());
    assert_eq!(error.to_string(), "inference failed: timeout");
}
