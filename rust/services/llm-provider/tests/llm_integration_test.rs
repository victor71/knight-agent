//! LLM Integration Test
//!
//! Tests actual LLM API calls. Requires valid API key in environment.

use std::collections::HashMap;

use llm_provider::{
    provider::{GenericLLMProvider, LLMProtocol, ModelPricing, ProviderConfig},
    ChatCompletionRequest, Content, Message, MessageRole,
    LLMProvider,
};

fn create_minimaxi_provider() -> GenericLLMProvider {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set");

    let mut model_pricing = HashMap::new();
    model_pricing.insert(
        "MiniMax-M2.7".to_string(),
        ModelPricing::new(0.0, 0.0), // Free tier
    );

    // Try Anthropic protocol with combined path
    // Full URL: https://api.minimaxi.com/anthropic/v1/messages
    let config = ProviderConfig {
        name: "minimaxi".to_string(),
        api_key,
        base_url: "https://api.minimaxi.com/anthropic/v1".to_string(),
        protocol: LLMProtocol::Anthropic,
        models: vec!["MiniMax-M2.7".to_string()],
        default_model: Some("MiniMax-M2.7".to_string()),
        timeout_secs: 120,
        model_pricing,
    };
    GenericLLMProvider::new(config).unwrap()
}

#[tokio::test]
async fn test_llm_chat_completion() {
    let provider = create_minimaxi_provider();

    let request = ChatCompletionRequest {
        model: "MiniMax-M2.7".to_string(),
        messages: vec![Message {
            role: MessageRole::User,
            content: Some(Content::Text("Hello, respond with just 'Hi'".to_string())),
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: 0.7,
        max_tokens: 2000,
        ..Default::default()
    };

    let response = provider.chat_completion(request).await;
    assert!(response.is_ok(), "LLM call failed: {:?}", response.err());

    let resp = response.unwrap();
    println!("LLM Response: {:?}", resp.content);
    assert!(resp.content.is_some(), "Response should have content");
}

#[tokio::test]
async fn test_llm_stream_completion() {
    let provider = create_minimaxi_provider();

    let request = ChatCompletionRequest {
        model: "MiniMax-M2.7".to_string(),
        messages: vec![Message {
            role: MessageRole::User,
            content: Some(Content::Text("Count from 1 to 3".to_string())),
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: 0.7,
        max_tokens: 100,
        stream: true,
        ..Default::default()
    };

    let stream_result = provider.stream_completion(request).await;
    assert!(stream_result.is_ok(), "Stream failed: {:?}", stream_result.err());

    use futures::StreamExt;
    let mut stream = stream_result.unwrap();
    let mut chunks = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Stream chunk failed");
        chunks.push(chunk);
    }

    println!("Streamed {} chunks", chunks.len());
    assert!(!chunks.is_empty(), "Should have received chunks");
}
