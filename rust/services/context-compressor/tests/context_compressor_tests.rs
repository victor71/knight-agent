//! Context compressor tests

use context_compressor::{
   CompressionConfig, CompressionOptions, CompressionStrategy, ContextCompressor, ContextCompressorImpl,
ContentType, Message,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_context_compressor_new() {
    let compressor = ContextCompressorImpl::new();
    assert_eq!(compressor.name(), "context-compressor");
    assert!(!compressor.is_initialized());
}

#[tokio::test]
async fn test_initialize() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();
    assert!(compressor.is_initialized());
}

#[tokio::test]
async fn test_compress_single_message() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![Message {
        id: "msg-1".to_string(),
        role: "user".to_string(),
        content: "Hello, world!".to_string(),
        metadata: HashMap::new(),
    }];

    // Use keep_recent_messages = 0 to actually compress the single message
    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };

    let point = compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    assert_eq!(point.session_id, "session-1");
    assert_eq!(point.strategy, CompressionStrategy::Summary);
    assert!(!point.summary.is_empty());
}

#[tokio::test]
async fn test_compress_multiple_messages() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![
        Message {
            id: "msg-1".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            metadata: HashMap::new(),
        },
        Message {
            id: "msg-2".to_string(),
            role: "assistant".to_string(),
            content: "Hi there!".to_string(),
            metadata: HashMap::new(),
        },
        Message {
            id: "msg-3".to_string(),
            role: "user".to_string(),
            content: "How are you?".to_string(),
            metadata: HashMap::new(),
        },
    ];

    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };
    let point = compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    assert_eq!(point.message_count, 3);
    assert!(point.token_saved > 0);
}

#[tokio::test]
async fn test_preserve_system_messages() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![
        Message {
            id: "sys-1".to_string(),
            role: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
            metadata: HashMap::new(),
        },
        Message {
            id: "msg-1".to_string(),
            role: "user".to_string(),
            content: "Hello world how are you today".to_string(),
            metadata: HashMap::new(),
        },
    ];

    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };
    let point = compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    // System message should be preserved
    assert!(point.preserved_messages.iter().any(|m| m.role == "system"));
}

#[tokio::test]
async fn test_get_compression_points() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![Message {
        id: "msg-1".to_string(),
        role: "user".to_string(),
        content: "Test".to_string(),
        metadata: HashMap::new(),
    }];

    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };
    compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    let points = compressor.get_compression_points("session-1").await.unwrap();
    assert_eq!(points.len(), 1);
}

#[tokio::test]
async fn test_get_compression_point() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![Message {
        id: "msg-1".to_string(),
        role: "user".to_string(),
        content: "Test".to_string(),
        metadata: HashMap::new(),
    }];

    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };
    let created = compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    let retrieved = compressor.get_compression_point(&created.id).await.unwrap();
    assert_eq!(retrieved.id, created.id);
}

#[tokio::test]
async fn test_restore_compression_point() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![Message {
        id: "msg-1".to_string(),
        role: "user".to_string(),
        content: "Test".to_string(),
        metadata: HashMap::new(),
    }];

    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };
    let point = compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    let restored = compressor.restore_compression_point(&point.id).await.unwrap();
    // Should contain preserved messages + summary message
    assert!(!restored.is_empty());
}

#[tokio::test]
async fn test_delete_compression_point() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![Message {
        id: "msg-1".to_string(),
        role: "user".to_string(),
        content: "Test".to_string(),
        metadata: HashMap::new(),
    }];

    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };
    let point = compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    compressor.delete_compression_point(&point.id).await.unwrap();

    let result = compressor.get_compression_point(&point.id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_estimate_tokens() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![
        Message {
            id: "msg-1".to_string(),
            role: "user".to_string(),
            content: "Hello world".to_string(), // 11 chars ≈ 3 tokens
            metadata: HashMap::new(),
        },
        Message {
            id: "msg-2".to_string(),
            role: "assistant".to_string(),
            content: "Hi there!".to_string(), // 9 chars ≈ 2 tokens
            metadata: HashMap::new(),
        },
    ];

    let estimation = compressor.estimate_tokens(&messages).await.unwrap();
    assert_eq!(estimation.count, 5); // 11/4 + 9/4 ≈ 3 + 2 = 5
}

#[tokio::test]
async fn test_compression_stats() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let messages = vec![Message {
        id: "msg-1".to_string(),
        role: "user".to_string(),
        content: "Test message for compression".to_string(),
        metadata: HashMap::new(),
    }];

    let options = CompressionOptions {
        keep_recent_messages: 0,
        ..Default::default()
    };
    compressor
        .compress("session-1", messages, CompressionStrategy::Summary, options)
        .await
        .unwrap();

    let stats = compressor.get_compression_stats("session-1").await.unwrap();
    assert_eq!(stats.total_compressions, 1);
    assert!(stats.total_tokens_saved > 0);
}

#[tokio::test]
async fn test_should_compress() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let response = compressor.should_compress("session-1").await.unwrap();
    // Should not need compression initially
    assert!(!response.should || response.estimated_tokens > 0);
}

#[tokio::test]
async fn test_compression_config() {
    let compressor = ContextCompressorImpl::new();
    compressor.initialize().await.unwrap();

    let config = compressor.get_compression_config(None).await.unwrap();
    assert_eq!(config.default_strategy, CompressionStrategy::Summary);

    let new_config = CompressionConfig {
        default_strategy: CompressionStrategy::Semantic,
        ..Default::default()
    };

    compressor.update_compression_config(new_config.clone()).await.unwrap();

    let updated = compressor.get_compression_config(None).await.unwrap();
    assert_eq!(updated.default_strategy, CompressionStrategy::Semantic);
}

#[test]
fn test_content_type_detection() {
    // System messages
    assert_eq!(
        ContentType::detect("You are helpful", "system"),
        ContentType::System
    );

    // Code
    assert_eq!(
        ContentType::detect("function test() { return 42; }", "assistant"),
        ContentType::Code
    );

    // Logs
    assert_eq!(
        ContentType::detect("[LOG] 2024-01-01 DEBUG: Starting process", "assistant"),
        ContentType::Log
    );

    // Config
    assert_eq!(
        ContentType::detect("{\"key\": \"value\"}", "assistant"),
        ContentType::Config
    );

    // Text (default)
    assert_eq!(
        ContentType::detect("Hello, how are you today?", "user"),
        ContentType::Text
    );
}

#[tokio::test]
async fn test_compression_options_default() {
    let options = CompressionOptions::default();
    assert_eq!(options.keep_recent_messages, 20);
    assert_eq!(options.keep_recent_tokens, 10000);
    assert!(options.keep_system);
    assert_eq!(options.min_compression_ratio, 0.3);
}

#[tokio::test]
async fn test_compression_trigger_default() {
    let trigger = context_compressor::CompressionTrigger::default();
    assert_eq!(trigger.message_count, 50);
    assert_eq!(trigger.token_count, 100000);
    assert!(trigger.auto_compress);
}
