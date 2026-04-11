//! Request Serialization Test
//!
//! Tests that the actual request format matches what's expected by the API.

use llm_provider::{ChatCompletionRequest, Content, Message, MessageRole};

#[test]
fn test_request_serialization() {
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

    let json = serde_json::to_string_pretty(&request).unwrap();
    println!("Serialized request:");
    println!("{}", json);

    // Verify the JSON structure
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Check model
    assert_eq!(value["model"], "MiniMax-M2.7");

    // Check messages array exists
    assert!(value["messages"].is_array());

    let messages = value["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let first_message = &messages[0];

    // Check role
    assert_eq!(first_message["role"], "user");

    // Check content - should be a string for Text content
    assert!(first_message["content"].is_string());
    assert_eq!(first_message["content"], "Hello, respond with just 'Hi'");

    // Check temperature and max_tokens
    assert_eq!(value["temperature"], 0.7);
    assert_eq!(value["max_tokens"], 2000);
}

#[test]
fn test_message_with_text_content_serialization() {
    let message = Message {
        role: MessageRole::User,
        content: Some(Content::Text("Test message".to_string())),
        tool_calls: None,
        tool_call_id: None,
    };

    let json = serde_json::to_string(&message).unwrap();
    println!("Serialized message with Text content:");
    println!("{}", json);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // For Text content, it should serialize to a plain string
    assert_eq!(value["role"], "user");
    assert_eq!(value["content"], "Test message");
}

#[test]
fn test_request_with_multiple_messages() {
    let request = ChatCompletionRequest {
        model: "MiniMax-M2.7".to_string(),
        messages: vec![
            Message {
                role: MessageRole::System,
                content: Some(Content::Text("You are a helpful assistant.".to_string())),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: MessageRole::User,
                content: Some(Content::Text("Hello!".to_string())),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        temperature: 0.7,
        max_tokens: 2000,
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&request).unwrap();
    println!("Serialized request with multiple messages:");
    println!("{}", json);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let messages = value["messages"].as_array().unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], "You are a helpful assistant.");
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "Hello!");
}
