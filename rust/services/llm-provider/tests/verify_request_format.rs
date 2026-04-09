//! Verify Request Format
//!
//! Verify that the request format matches API expectations.

use llm_provider::{
    ChatCompletionRequest, Content, Message, MessageRole,
};

#[test]
fn verify_anthropic_api_format() {
    // This is what Anthropic API expects:
    // https://docs.anthropic.com/en/api/messages
    let request = ChatCompletionRequest {
        model: "claude-sonnet-4-6".to_string(),
        messages: vec![
            Message {
                role: MessageRole::User,
                content: Some(Content::Text("Hello".to_string())),
                tool_calls: None,
                tool_call_id: None,
            }
        ],
        temperature: 0.7,
        max_tokens: 2000,
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&request).unwrap();
    println!("Request format:");
    println!("{}", json);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify Anthropic API format
    assert_eq!(value["model"], "claude-sonnet-4-6");
    assert_eq!(value["messages"][0]["role"], "user");
    assert_eq!(value["messages"][0]["content"], "Hello");
    assert_eq!(value["temperature"], 0.7);
    assert_eq!(value["max_tokens"], 2000);

    println!("✅ Request format matches Anthropic API specification");
}

#[test]
fn verify_message_role_lowercase() {
    let roles = vec![
        (MessageRole::System, "system"),
        (MessageRole::User, "user"),
        (MessageRole::Assistant, "assistant"),
        (MessageRole::Tool, "tool"),
    ];

    for (role, expected) in roles {
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
        println!("✅ {:?} serializes to \"{}\"", role, expected);

        // Test round-trip
        let parsed: MessageRole = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, role);
    }
}
