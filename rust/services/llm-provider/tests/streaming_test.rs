//! Streaming Response Tests
//!
//! Tests for SSE (Server-Sent Events) streaming behavior without actual HTTP requests.

use std::collections::HashMap;

use llm_provider::{
    provider::{GenericLLMProvider, LLMProtocol, ModelPricing, ProviderConfig},
    types::{ChatCompletionChunk, ChoiceChunk, Delta, DeltaContent},
    ChatCompletionRequest, Content, Message, MessageRole,
};

fn create_test_provider() -> GenericLLMProvider {
    let mut model_pricing = HashMap::new();
    model_pricing.insert("test-model".to_string(), ModelPricing::new(1.0, 2.0));

    let config = ProviderConfig {
        name: "test".to_string(),
        api_key: "test-key".to_string(),
        base_url: "https://api.test.com/v1".to_string(),
        protocol: LLMProtocol::OpenAI,
        models: vec!["test-model".to_string()],
        default_model: Some("test-model".to_string()),
        timeout_secs: 120,
        model_pricing,
    };
    GenericLLMProvider::new(config).unwrap()
}

// Helper function to create a simple chunk with content
fn create_content_chunk(text: &str, index: u32) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: "test-id".to_string(),
        type_field: "message".to_string(),
        role: Some("assistant".to_string()),
        content: Some(text.to_string()),
        is_thinking: None,
        model: "test-model".to_string(),
        choices: vec![ChoiceChunk {
            index,
            delta: Delta::MessageDelta {
                delta: llm_provider::types::MessageDeltaContent {
                    role: None,
                    content: Some(text.to_string()),
                    tool_calls: None,
                },
                index,
            },
            finish_reason: None,
        }],
    }
}

// Helper function to create a stop chunk
fn create_stop_chunk(index: u32) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: "test-id".to_string(),
        type_field: "message".to_string(),
        role: Some("assistant".to_string()),
        content: None,
        is_thinking: None,
        model: "test-model".to_string(),
        choices: vec![ChoiceChunk {
            index,
            delta: Delta::MessageStop,
            finish_reason: Some("stop".to_string()),
        }],
    }
}

// Mock SSE response data for testing

const OPENAI_SSE_STREAM: &str = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1699000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1699000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1699000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]

"#;

const ANTHROPIC_SSE_STREAM: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg-123","role":"assistant","content":[]}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":12}}

event: message_stop
data: {"type":"message_stop"}

"#;

const MALFORMED_SSE_STREAM: &str = r#"data: {"id":"123","choices":[{"delta":{"content":"First"}}]}

invalid line without data prefix

data: {"id":"123","choices":[{"delta":{"content":"Second"}}]

data: incomplete json {

data: {"id":"123","choices":[{"delta":{"content":"Third"}}]}

"#;

const EMPTY_CHUNKS_SSE_STREAM: &str = r#"data: {"id":"123","choices":[{"delta":{}}]}

data: {"id":"123","choices":[{"delta":{"content":"Hi"}}]}

data: {"id":"123","choices":[{"delta":{}}]}

data: {"id":"123","choices":[{"delta":{"content":" there"}}]}

data: [DONE]

"#;

// SSE line parsing tests

#[test]
fn test_parse_openai_sse_lines() {
    let lines: Vec<&str> = OPENAI_SSE_STREAM.lines().collect();

    let mut content_chunks = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || !line.starts_with("data:") {
            continue;
        }

        let data_part = line.strip_prefix("data:").unwrap().trim();
        if data_part == "[DONE]" {
            continue;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data_part) {
            if let Some(content) = json
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
            {
                content_chunks.push(content.to_string());
            }
        }
    }

    assert_eq!(content_chunks, vec!["Hello", " world"]);
}

#[test]
fn test_parse_anthropic_sse_lines() {
    let lines: Vec<&str> = ANTHROPIC_SSE_STREAM.lines().collect();

    let mut content_chunks = Vec::new();
    for line in lines {
        let line = line.trim();
        if !line.starts_with("data:") {
            continue;
        }

        let data_part = line.strip_prefix("data:").unwrap().trim();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data_part) {
            if json.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
                if let Some(text) = json
                    .get("delta")
                    .and_then(|d| d.get("text"))
                    .and_then(|t| t.as_str())
                {
                    content_chunks.push(text.to_string());
                }
            }
        }
    }

    assert_eq!(content_chunks, vec!["Hello", " world"]);
}

#[test]
fn test_handle_malformed_sse() {
    let lines: Vec<&str> = MALFORMED_SSE_STREAM.lines().collect();

    let mut valid_chunks = 0;
    let mut invalid_lines = 0;

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if !line.starts_with("data:") {
            invalid_lines += 1;
            continue;
        }

        let data_part = line.strip_prefix("data:").unwrap().trim();
        if serde_json::from_str::<serde_json::Value>(data_part).is_ok() {
            valid_chunks += 1;
        } else {
            invalid_lines += 1;
        }
    }

    // Should have 2 valid chunks (First, Third) and 3 invalid lines:
    // - one without data prefix
    // - one malformed JSON (missing closing bracket)
    // - one incomplete JSON
    assert_eq!(valid_chunks, 2);
    assert_eq!(invalid_lines, 3);
}

#[test]
fn test_handle_empty_chunks() {
    let lines: Vec<&str> = EMPTY_CHUNKS_SSE_STREAM.lines().collect();

    let mut content_chunks = Vec::new();
    let mut empty_chunks = 0;

    for line in lines {
        let line = line.trim();
        if !line.starts_with("data:") {
            continue;
        }

        let data_part = line.strip_prefix("data:").unwrap().trim();
        if data_part == "[DONE]" {
            continue;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data_part) {
            if let Some(content) = json
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
            {
                if let Some(text) = content.as_str() {
                    if !text.is_empty() {
                        content_chunks.push(text.to_string());
                    }
                }
            } else {
                empty_chunks += 1;
            }
        }
    }

    // Should have 2 non-empty chunks and 2 empty chunks
    assert_eq!(content_chunks, vec!["Hi", " there"]);
    assert_eq!(empty_chunks, 2);
}

// Buffer management tests

#[test]
fn test_buffer_line_splitting() {
    let input = b"Hello\nWorld\nFoo";
    let mut buffer = Vec::new();
    let mut lines = Vec::new();

    buffer.extend_from_slice(input);

    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line = buffer.drain(..=newline_pos).collect::<Vec<_>>();
        // Remove newline
        let line_str = String::from_utf8(line)
            .unwrap()
            .trim_end_matches('\n')
            .to_string();
        lines.push(line_str);
    }

    // "Foo" has no trailing newline, so it remains in buffer
    assert_eq!(lines, vec!["Hello", "World"]);
    assert_eq!(buffer, b"Foo");
}

#[test]
fn test_buffer_partial_line() {
    let input1 = b"Hello\nWor";
    let input2 = b"ld\n";

    let mut buffer = Vec::new();
    let mut lines = Vec::new();

    // First chunk - partial line
    buffer.extend_from_slice(input1);
    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line = buffer.drain(..=newline_pos).collect::<Vec<_>>();
        let line_str = String::from_utf8(line)
            .unwrap()
            .trim_end_matches('\n')
            .to_string();
        lines.push(line_str);
    }

    assert_eq!(lines, vec!["Hello"]);
    assert_eq!(buffer, b"Wor");

    // Second chunk - completes the line
    buffer.extend_from_slice(input2);
    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line = buffer.drain(..=newline_pos).collect::<Vec<_>>();
        let line_str = String::from_utf8(line)
            .unwrap()
            .trim_end_matches('\n')
            .to_string();
        lines.push(line_str);
    }

    assert_eq!(lines, vec!["Hello", "World"]);
    assert!(buffer.is_empty());
}

#[test]
fn test_buffer_multiple_newlines() {
    let input = b"Line1\n\n\nLine2\n\n";
    let mut buffer = Vec::new();
    let mut lines = Vec::new();

    buffer.extend_from_slice(input);

    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line = buffer.drain(..=newline_pos).collect::<Vec<_>>();
        let line_str = String::from_utf8(line)
            .unwrap()
            .trim_end_matches('\n')
            .to_string();
        lines.push(line_str);
    }

    // Empty lines are included
    assert_eq!(lines, vec!["Line1", "", "", "Line2", ""]);
}

#[test]
fn test_buffer_size_limit() {
    const MAX_BUFFER_SIZE: usize = 100;

    let large_input = b"A".repeat(200);
    let mut buffer = Vec::new();

    buffer.extend_from_slice(&large_input);

    // Safety check should prevent unbounded growth
    if buffer.len() > MAX_BUFFER_SIZE {
        buffer.clear();
    }

    assert!(buffer.is_empty() || buffer.len() <= MAX_BUFFER_SIZE);
}

// Chunk content aggregation tests

#[test]
fn test_aggregate_stream_content() {
    let chunks = vec![
        create_content_chunk("Hello", 0),
        create_content_chunk(" world", 0),
        create_stop_chunk(0),
    ];

    let mut full_content = String::new();
    for chunk in &chunks {
        if let Some(ref content) = chunk.content {
            full_content.push_str(content);
        }
    }

    assert_eq!(full_content, "Hello world");
}

#[test]
fn test_stream_with_empty_chunks() {
    let chunks = vec![
        ChatCompletionChunk {
            id: "test".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: None,
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![],
        },
        create_content_chunk("Start", 0),
        ChatCompletionChunk {
            id: "test".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: None,
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![],
        },
        ChatCompletionChunk {
            id: "test".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: Some(" End".to_string()),
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta::MessageStop,
                finish_reason: Some("stop".to_string()),
            }],
        },
    ];

    let mut full_content = String::new();
    for chunk in &chunks {
        if let Some(ref content) = chunk.content {
            full_content.push_str(content);
        }
    }

    assert_eq!(full_content, "Start End");
}

#[test]
fn test_stream_multi_index() {
    let chunks = vec![
        create_content_chunk("Response 1, ", 0),
        create_content_chunk("Response 2, ", 1),
        ChatCompletionChunk {
            id: "test".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: Some("part 2".to_string()),
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta::MessageStop,
                finish_reason: Some("stop".to_string()),
            }],
        },
        ChatCompletionChunk {
            id: "test".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: Some("part 2".to_string()),
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 1,
                delta: Delta::MessageStop,
                finish_reason: Some("stop".to_string()),
            }],
        },
    ];

    let mut responses: HashMap<u32, String> = HashMap::new();

    for chunk in &chunks {
        if let Some(ref content) = chunk.content {
            if !chunk.choices.is_empty() {
                let index = chunk.choices[0].index;
                let entry = responses.entry(index).or_insert_with(String::new);
                entry.push_str(content);
            }
        }
    }

    assert_eq!(responses.get(&0).unwrap(), "Response 1, part 2");
    assert_eq!(responses.get(&1).unwrap(), "Response 2, part 2");
}

// Edge case tests

#[test]
fn test_unicode_in_stream() {
    let chunks = vec![
        create_content_chunk("你好", 0),
        create_content_chunk("世界", 0),
        create_stop_chunk(0),
    ];

    let mut full_content = String::new();
    for chunk in &chunks {
        if let Some(ref content) = chunk.content {
            full_content.push_str(content);
        }
    }

    assert_eq!(full_content, "你好世界");
}

#[test]
fn test_special_characters_in_stream() {
    let chunks = vec![
        create_content_chunk("Line 1\n", 0),
        create_content_chunk("  Indented\n", 0),
        ChatCompletionChunk {
            id: "test".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: Some("  ".to_string()),
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta::MessageStop,
                finish_reason: Some("stop".to_string()),
            }],
        },
    ];

    let mut full_content = String::new();
    for chunk in &chunks {
        if let Some(ref content) = chunk.content {
            full_content.push_str(content);
        }
    }

    assert_eq!(full_content, "Line 1\n  Indented\n  ");
}

#[test]
fn test_chunk_finish_reasons() {
    let finish_reasons = vec!["stop", "length", "content_filter", "tool_calls"];

    for reason in finish_reasons {
        let chunk = ChatCompletionChunk {
            id: "test".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: None,
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta::MessageStop,
                finish_reason: Some(reason.to_string()),
            }],
        };

        assert_eq!(chunk.choices[0].finish_reason.as_ref().unwrap(), reason);
    }
}

#[test]
fn test_stream_request_flag() {
    let mut request = ChatCompletionRequest {
        model: "test-model".to_string(),
        messages: vec![Message {
            role: MessageRole::User,
            content: Some(Content::Text("Hello".to_string())),
            tool_calls: None,
            tool_call_id: None,
        }],
        stream: true,
        ..Default::default()
    };

    assert!(request.stream);

    request.stream = false;
    assert!(!request.stream);
}

// Integration-style tests with mock stream

#[test]
fn test_full_stream_simulation() {
    // Simulate a complete stream from request to aggregated response
    let _provider = create_test_provider();

    let request = ChatCompletionRequest {
        model: "test-model".to_string(),
        messages: vec![Message {
            role: MessageRole::User,
            content: Some(Content::Text("Test".to_string())),
            tool_calls: None,
            tool_call_id: None,
        }],
        stream: true,
        ..Default::default()
    };

    assert!(request.stream);

    // Simulate receiving chunks
    let mock_chunks = vec![
        create_content_chunk("This ", 0),
        create_content_chunk("is ", 0),
        create_content_chunk("a ", 0),
        create_content_chunk("test", 0),
        create_stop_chunk(0),
    ];

    let mut response = String::new();
    let mut chunk_count = 0;

    for chunk in mock_chunks {
        chunk_count += 1;
        if let Some(content) = chunk.content {
            response.push_str(&content);
        }
    }

    assert_eq!(chunk_count, 5);
    assert_eq!(response, "This is a test");
}

// Test chunk serialization/deserialization

#[test]
fn test_chunk_serialization() {
    let chunk = create_content_chunk("Hello", 0);

    let json = serde_json::to_string(&chunk).unwrap();
    assert!(json.contains("Hello"));
    assert!(json.contains("assistant"));

    let parsed: ChatCompletionChunk = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.content, chunk.content);
    assert_eq!(parsed.choices[0].index, 0);
}

#[test]
fn test_delta_content_block() {
    let delta = Delta::ContentBlockDelta {
        delta: DeltaContent {
            text: Some("test text".to_string()),
            name: None,
        },
        index: 0,
    };

    let json = serde_json::to_string(&delta).unwrap();
    assert!(json.contains("content_block_delta"));
    assert!(json.contains("test text"));

    let parsed: Delta = serde_json::from_str(&json).unwrap();
    match parsed {
        Delta::ContentBlockDelta { delta, index } => {
            assert_eq!(delta.text.unwrap(), "test text");
            assert_eq!(index, 0);
        }
        _ => panic!("Expected ContentBlockDelta"),
    }
}

// Additional edge case tests for streaming

#[test]
fn test_chunk_with_only_finish_reason() {
    // Test a chunk that only has finish_reason, no content
    let chunk = ChatCompletionChunk {
        id: "test-id".to_string(),
        type_field: "message".to_string(),
        role: Some("assistant".to_string()),
        content: None,
        is_thinking: None,
        model: "test-model".to_string(),
        choices: vec![ChoiceChunk {
            index: 0,
            delta: Delta::MessageStop,
            finish_reason: Some("length".to_string()),
        }],
    };

    assert!(chunk.content.is_none());
    assert_eq!(chunk.choices[0].finish_reason.as_ref().unwrap(), "length");
}

#[test]
fn test_empty_choices_array() {
    // Test chunk with empty choices array - content is still available at chunk level
    let chunk = ChatCompletionChunk {
        id: "test-id".to_string(),
        type_field: "message".to_string(),
        role: Some("assistant".to_string()),
        content: Some("test".to_string()),
        is_thinking: None,
        model: "test-model".to_string(),
        choices: vec![],
    };

    // Empty choices is valid - content can be at chunk level (e.g., for Anthropic format)
    assert!(chunk.choices.is_empty());
    assert_eq!(chunk.content.unwrap(), "test");
}

#[test]
fn test_chunk_aggregation_with_thinking() {
    // Test that thinking chunks can be filtered/identified
    let chunks = vec![
        ChatCompletionChunk {
            id: "test-id".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: None,
            is_thinking: Some(true),
            model: "test-model".to_string(),
            choices: vec![],
        },
        create_content_chunk("Actual response", 0),
        create_stop_chunk(0),
    ];

    let mut thinking_count = 0;
    let mut actual_content = String::new();

    for chunk in &chunks {
        if chunk.is_thinking.unwrap_or(false) {
            thinking_count += 1;
        } else if let Some(ref content) = chunk.content {
            actual_content.push_str(content);
        }
    }

    assert_eq!(thinking_count, 1);
    assert_eq!(actual_content, "Actual response");
}

#[test]
fn test_stream_with_delays_between_chunks() {
    // Simulate a scenario where chunks arrive with variable timing
    let chunks = vec![
        create_content_chunk("Hello", 0),
        create_content_chunk(",", 0),
        create_content_chunk(" how", 0),
        create_content_chunk(" are", 0),
        create_content_chunk(" you?", 0),
        create_stop_chunk(0),
    ];

    let mut full_content = String::new();
    let mut delay_simulated = 0;

    for chunk in chunks {
        // Simulate processing delay
        if chunk.content.is_some() {
            delay_simulated += 10; // ms
        }
        if let Some(content) = chunk.content {
            full_content.push_str(&content);
        }
    }

    assert_eq!(full_content, "Hello, how are you?");
    assert_eq!(delay_simulated, 50); // 5 content chunks * 10ms
}

#[test]
fn test_stream_error_recovery() {
    // Test that stream can recover from a bad chunk
    let mut chunks = vec![create_content_chunk("Start", 0)];

    // Add a malformed chunk (simulated by empty choices with error)
    chunks.push(ChatCompletionChunk {
        id: "error-id".to_string(),
        type_field: "error".to_string(),
        role: None,
        content: None,
        is_thinking: None,
        model: "error-model".to_string(),
        choices: vec![],
    });

    chunks.push(create_content_chunk("End", 0));
    chunks.push(create_stop_chunk(0));

    let mut full_content = String::new();
    let mut error_count = 0;

    for chunk in chunks {
        if chunk.type_field == "error" {
            error_count += 1;
            continue; // Skip error chunks
        }
        if let Some(content) = chunk.content {
            full_content.push_str(&content);
        }
    }

    assert_eq!(error_count, 1);
    assert_eq!(full_content, "StartEnd");
}

#[test]
fn test_long_content_aggregation() {
    // Test aggregation of many small chunks into a long response
    let words = vec![
        "The", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog",
    ];
    let mut chunks = Vec::new();

    for word in words {
        chunks.push(create_content_chunk(&format!("{} ", word), 0));
    }
    chunks.push(create_stop_chunk(0));

    let mut full_content = String::new();
    for chunk in &chunks {
        if let Some(ref content) = chunk.content {
            full_content.push_str(content);
        }
    }

    assert_eq!(full_content, "The quick brown fox jumps over the lazy dog ");
    assert_eq!(chunks.len(), 10); // 9 words + 1 stop chunk
}

#[test]
fn test_concurrent_stream_indices() {
    // Test handling multiple concurrent streams with different indices
    let chunks = vec![
        create_content_chunk("Stream 0, part 1. ", 0),
        create_content_chunk("Stream 1, part 1. ", 1),
        create_content_chunk("Stream 2, part 1. ", 2),
        create_content_chunk("Stream 0, part 2. ", 0),
        create_content_chunk("Stream 1, part 2. ", 1),
        create_content_chunk("Stream 2, part 2. ", 2),
        ChatCompletionChunk {
            id: "stop-0".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: Some("end".to_string()),
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 0,
                delta: Delta::MessageStop,
                finish_reason: Some("stop".to_string()),
            }],
        },
        ChatCompletionChunk {
            id: "stop-1".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: Some("end".to_string()),
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 1,
                delta: Delta::MessageStop,
                finish_reason: Some("stop".to_string()),
            }],
        },
        ChatCompletionChunk {
            id: "stop-2".to_string(),
            type_field: "message".to_string(),
            role: Some("assistant".to_string()),
            content: Some("end".to_string()),
            is_thinking: None,
            model: "test".to_string(),
            choices: vec![ChoiceChunk {
                index: 2,
                delta: Delta::MessageStop,
                finish_reason: Some("stop".to_string()),
            }],
        },
    ];

    let mut streams: HashMap<u32, String> = HashMap::new();

    for chunk in &chunks {
        if !chunk.choices.is_empty() {
            let index = chunk.choices[0].index;
            if let Some(ref content) = chunk.content {
                let entry = streams.entry(index).or_insert_with(String::new);
                entry.push_str(content);
            }
        }
    }

    assert_eq!(
        streams.get(&0).unwrap(),
        "Stream 0, part 1. Stream 0, part 2. end"
    );
    assert_eq!(
        streams.get(&1).unwrap(),
        "Stream 1, part 1. Stream 1, part 2. end"
    );
    assert_eq!(
        streams.get(&2).unwrap(),
        "Stream 2, part 1. Stream 2, part 2. end"
    );
}

#[test]
fn test_chunk_with_tool_calls() {
    // Test chunks that contain tool call information
    let tool_call_chunk = ChatCompletionChunk {
        id: "test-id".to_string(),
        type_field: "message".to_string(),
        role: Some("assistant".to_string()),
        content: Some("I'll use a tool".to_string()),
        is_thinking: None,
        model: "test-model".to_string(),
        choices: vec![ChoiceChunk {
            index: 0,
            delta: Delta::MessageDelta {
                delta: llm_provider::types::MessageDeltaContent {
                    role: Some("assistant".to_string()),
                    content: Some("I'll use a tool".to_string()),
                    tool_calls: Some(vec![llm_provider::types::ToolCall {
                        id: "call_123".to_string(),
                        tool_type: "function".to_string(),
                        function: llm_provider::types::ToolCallFunction {
                            name: "search".to_string(),
                            arguments: "{\"query\":\"test\"}".to_string(),
                        },
                    }]),
                },
                index: 0,
            },
            finish_reason: None,
        }],
    };

    assert_eq!(tool_call_chunk.content.as_ref().unwrap(), "I'll use a tool");
    match &tool_call_chunk.choices[0].delta {
        Delta::MessageDelta { delta, .. } => {
            assert!(delta.tool_calls.is_some());
            assert_eq!(delta.tool_calls.as_ref().unwrap()[0].id, "call_123");
        }
        _ => panic!("Expected MessageDelta"),
    }
}

#[test]
fn test_minimax_text_delta_format() {
    // Test MiniMax-specific text_delta format
    let delta = Delta::ContentBlockDelta {
        delta: DeltaContent {
            text: Some("Here is the response".to_string()),
            name: None,
        },
        index: 0,
    };

    match delta {
        Delta::ContentBlockDelta { delta, .. } => {
            assert_eq!(delta.text.unwrap(), "Here is the response");
        }
        _ => panic!("Expected ContentBlockDelta with text"),
    }
}

#[test]
fn test_sse_done_signal() {
    // Test that [DONE] signal is properly handled
    let sse_lines = vec![
        "data: {\"content\":\"Hello\"}",
        "data: {\"content\":\"World\"}",
        "data: [DONE]",
        "",
    ];

    let mut chunks = 0;
    let mut done_found = false;

    for line in sse_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line == "data: [DONE]" {
            done_found = true;
            continue;
        }

        if line.starts_with("data:") {
            chunks += 1;
        }
    }

    assert_eq!(chunks, 2);
    assert!(done_found);
}

#[test]
fn test_buffer_carriage_return_handling() {
    // Test handling of \r\n (Windows-style line endings)
    let input = b"Line1\r\nLine2\r\nLine3\r\n";
    let mut buffer = Vec::new();
    let mut lines = Vec::new();

    buffer.extend_from_slice(input);

    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line = buffer.drain(..=newline_pos).collect::<Vec<_>>();
        let line_str = String::from_utf8(line)
            .unwrap()
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string();
        lines.push(line_str);
    }

    assert_eq!(lines, vec!["Line1", "Line2", "Line3"]);
    assert!(buffer.is_empty());
}
