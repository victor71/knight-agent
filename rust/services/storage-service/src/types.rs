//! Storage Types
//!
//! Core data types for the storage service.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "type")]
pub enum SessionStatus {
    #[default]
    Active,
    Archived,
    Deleted,
}

/// Session data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub status: SessionStatus,
    pub workspace_root: String,
    pub project_type: Option<String>,
    pub created_at: i64,
    pub last_active_at: i64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Message data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: i64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Compression point data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionPoint {
    pub id: String,
    pub session_id: String,
    pub created_at: i64,
    pub before_count: i64,
    pub after_count: i64,
    pub summary: String,
    pub token_saved: i64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TaskStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub workflow_id: Option<String>,
    pub name: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub agent_id: Option<String>,
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub outputs: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

/// Task update structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub status: Option<TaskStatus>,
    pub input: Option<HashMap<String, serde_json::Value>>,
    pub output: Option<HashMap<String, serde_json::Value>>,
    pub error: Option<String>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub created_at: i64,
}

/// Session filter
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionFilter {
    pub status: Option<SessionStatus>,
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
    pub workspace: Option<String>,
}

/// Task filter
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskFilter {
    pub workflow_id: Option<String>,
    pub status: Option<TaskStatus>,
    pub task_type: Option<String>,
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub sessions: SessionStats,
    pub messages: MessageStats,
    pub tasks: TaskStats,
    pub database_size_mb: f64,
    pub compression_ratio: f64,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStats {
    pub total: i64,
    pub active: i64,
    pub archived: i64,
    pub total_messages: i64,
}

/// Message statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageStats {
    pub total: i64,
    pub by_role: HashMap<String, i64>,
    pub avg_tokens: f64,
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskStats {
    pub total: i64,
    pub by_status: HashMap<String, i64>,
    pub by_type: HashMap<String, i64>,
}

/// Token statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenStats {
    pub total: i64,
    pub input: i64,
    pub output: i64,
    pub cost_estimate: f64,
}

/// Session usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionUsageStats {
    pub new_count: i64,
    pub active_count: i64,
    pub total_count: i64,
    pub messages_total: i64,
}

/// Agent usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentUsageStats {
    pub llm_calls: i64,
    pub active_count: i64,
    pub created_count: i64,
}

/// System usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemUsageStats {
    pub memory_mb_avg: f64,
    pub memory_mb_peak: i64,
    pub cpu_avg: f64,
    pub uptime_seconds: i64,
}

/// Statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSnapshot {
    pub id: String,
    pub period: String,
    pub timestamp_start: i64,
    pub timestamp_end: i64,
    pub created_at: i64,
    pub tokens: TokenStats,
    pub sessions: SessionUsageStats,
    pub agents: AgentUsageStats,
    pub system: SystemUsageStats,
}

/// Token usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    pub id: String,
    pub session_id: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cost_estimate: f64,
    pub timestamp: i64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// LLM call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMCallRecord {
    pub id: String,
    pub session_id: String,
    pub agent_id: Option<String>,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub latency_ms: Option<i64>,
    pub timestamp: i64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Session event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: String,
    pub session_id: String,
    pub event_type: String,
    pub timestamp: i64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Daily report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub date: String,
    pub tokens: TokenStats,
    pub sessions: SessionUsageStats,
    pub agents: AgentUsageStats,
    #[serde(default)]
    pub by_hour: Vec<StatsSnapshot>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub database_path: String,
    pub wal_enabled: bool,
    pub cache_size: i64,
    pub page_size: i64,
    pub backup_enabled: bool,
    pub backup_interval_secs: i64,
    pub backup_retention_days: i64,
    pub backup_path: String,
    pub vacuum_interval_secs: i64,
    pub reindex_interval_secs: i64,
    pub auto_vacuum: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_path: "./storage/knight-agent.db".to_string(),
            wal_enabled: true,
            cache_size: 10000,
            page_size: 4096,
            backup_enabled: true,
            backup_interval_secs: 86400,
            backup_retention_days: 7,
            backup_path: "./storage/backups".to_string(),
            vacuum_interval_secs: 604800,
            reindex_interval_secs: 1209600,
            auto_vacuum: true,
        }
    }
}
