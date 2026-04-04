//! Public types for context compressor

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compression strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum CompressionStrategy {
    #[default]
    Summary,  // LLM-based summarization
    Semantic, // Semantic clustering and extraction
    Hybrid,   // Combination of summary and semantic
}


/// Message structure for compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Compression point - saved state after compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionPoint {
    pub id: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub strategy: CompressionStrategy,
    // Compression range
    pub message_range: MessageRange,
    pub message_count: usize,
    pub token_count: usize,
    // Compression result
    pub summary: String,
    pub preserved_messages: Vec<Message>,
    // Statistics
    pub original_size: usize,
    pub compressed_size: usize,
    pub token_saved: usize,
    pub compression_ratio: f64,
    // Messages replaced by this compression point
    pub replaced_message_ids: Vec<String>,
}

/// Message range in the original conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRange {
    pub start: usize,
    pub end: usize,
}

/// Compression before state
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionBefore {
    pub message_count: usize,
    pub token_count: usize,
    pub message_range: MessageRange,
}

/// Compression after state
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionAfter {
    pub message_count: usize,
    pub token_count: usize,
    pub messages: Vec<Message>,
}

/// Compression options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionOptions {
    #[serde(default = "default_keep_recent_messages")]
    pub keep_recent_messages: usize,
    #[serde(default = "default_keep_recent_tokens")]
    pub keep_recent_tokens: usize,
    #[serde(default = "default_keep_system")]
    pub keep_system: bool,
    #[serde(default)]
    pub preserve_keys: Vec<String>,
    #[serde(default = "default_min_compression_ratio")]
    pub min_compression_ratio: f64,
}

fn default_keep_recent_messages() -> usize { 20 }
fn default_keep_recent_tokens() -> usize { 10000 }
fn default_keep_system() -> bool { true }
fn default_min_compression_ratio() -> f64 { 0.3 }

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            keep_recent_messages: 20,
            keep_recent_tokens: 10000,
            keep_system: true,
            preserve_keys: Vec::new(),
            min_compression_ratio: 0.3,
        }
    }
}

/// Compression trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionTrigger {
    #[serde(default = "default_message_count")]
    pub message_count: usize,
    #[serde(default = "default_token_count")]
    pub token_count: usize,
    #[serde(default = "default_auto_compress")]
    pub auto_compress: bool,
}

fn default_message_count() -> usize { 50 }
fn default_token_count() -> usize { 100000 }
fn default_auto_compress() -> bool { true }

impl Default for CompressionTrigger {
    fn default() -> Self {
        Self {
            message_count: 50,
            token_count: 100000,
            auto_compress: true,
        }
    }
}

/// Full compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CompressionConfig {
    #[serde(default)]
    pub trigger: CompressionTrigger,
    #[serde(default)]
    pub default_strategy: CompressionStrategy,
    #[serde(default)]
    pub default_options: CompressionOptions,
}


/// Compression history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionHistoryEntry {
    pub point_id: String,
    pub timestamp: DateTime<Utc>,
    pub strategy: CompressionStrategy,
    pub token_saved: usize,
    pub compression_ratio: f64,
}

/// Compression statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub total_compressions: usize,
    pub total_tokens_saved: usize,
    pub avg_compression_ratio: f64,
    pub last_compression: Option<DateTime<Utc>>,
    pub compression_history: Vec<CompressionHistoryEntry>,
    // Token budget tracking
    pub tokens_used: usize,
    pub budget_limit: usize,
    pub budget_remaining: usize,
    pub compression_paused_until: Option<DateTime<Utc>>,
    pub last_budget_reset: Option<DateTime<Utc>>,
}

impl Default for CompressionStats {
    fn default() -> Self {
        Self {
            total_compressions: 0,
            total_tokens_saved: 0,
            avg_compression_ratio: 0.0,
            last_compression: None,
            compression_history: Vec::new(),
            tokens_used: 0,
            budget_limit: 100000,
            budget_remaining: 100000,
            compression_paused_until: None,
            last_budget_reset: Some(Utc::now()),
        }
    }
}

/// Compression job status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Compression job for async operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionJob {
    pub job_id: String,
    pub session_id: String,
    pub status: CompressionJobStatus,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Compressed context storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedContext {
    pub id: String,
    pub original_size: usize,
    pub compressed_size: usize,
    pub data: Vec<u8>,
}

/// Should compress response
#[derive(Debug, Clone)]
pub struct ShouldCompressResponse {
    pub should: bool,
    pub reason: String,
    pub estimated_tokens: usize,
}

/// Token estimation result
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct TokenEstimation {
    pub count: usize,
    pub by_message: Vec<usize>,
}

