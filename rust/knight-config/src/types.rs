//! Configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main Knight Agent configuration (knight.json)
/// This file only contains user-facing LLM configuration.
/// Other system configurations are in config/*.yaml files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnightConfig {
    /// LLM provider configuration (user-facing)
    pub llm: Option<LlmConfig>,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    /// Default provider name
    pub default_provider: Option<String>,
    /// LLM providers
    pub providers: HashMap<String, LlmProviderConfig>,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderConfig {
    /// Provider type (openai, anthropic)
    #[serde(rename = "type")]
    pub provider_type: String,
    /// API key (supports ${ENV_VAR} syntax)
    pub api_key: String,
    /// Base URL
    pub base_url: String,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Available models
    pub models: Vec<LlmModelConfig>,
    /// Default model for this provider
    pub default_model: String,
}

fn default_timeout() -> u64 {
    120
}

/// LLM model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmModelConfig {
    /// Model ID
    pub id: String,
    /// Context length in tokens
    pub context_length: usize,
    /// Pricing information
    pub pricing: LlmPricing,
    /// Model capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// LLM pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmPricing {
    /// Input price per 1M tokens
    pub input: f64,
    /// Output price per 1M tokens
    pub output: f64,
    /// Currency code
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    /// Database path
    pub database_path: Option<String>,
    /// Maximum database size in MB
    #[serde(default = "default_max_db_size")]
    pub max_db_size_mb: usize,
}

fn default_max_db_size() -> usize {
    1024
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    /// Sandbox enabled
    #[serde(default = "default_sandbox")]
    pub sandbox_enabled: bool,
    /// Allowed operations
    #[serde(default)]
    pub allowed_operations: Vec<String>,
    /// Blocked operations
    #[serde(default)]
    pub blocked_operations: Vec<String>,
}

fn default_sandbox() -> bool {
    true
}

/// Agent configuration (config/agent.yaml)
///
/// This consolidates configurations from:
/// - agent-runtime (execution limits, retry, timeout, streaming)
/// - skill-engine (directories, execution, triggers, llm_parsing)
/// - task-manager (execution, retry, storage, dag)
/// - workflows-directory (directories, execution, versioning, cache)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    // ========== Common Settings ==========
    /// Default agent variant
    pub default_variant: Option<String>,
    /// Maximum concurrent tasks
    #[serde(default = "default_max_tasks")]
    pub max_concurrent_tasks: usize,
    /// Task timeout in seconds
    #[serde(default = "default_task_timeout")]
    pub task_timeout_secs: u64,

    // ========== Agent Runtime Settings ==========
    /// Agent runtime configuration
    #[serde(default)]
    pub runtime: AgentRuntimeConfig,
    /// Skill engine configuration
    #[serde(default)]
    pub skill: SkillEngineConfig,
    /// Task manager configuration
    #[serde(default)]
    pub task: TaskManagerConfig,
    /// Workflow configuration
    #[serde(default)]
    pub workflow: WorkflowConfig,
}

/// Agent runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeConfig {
    /// Maximum execution time in seconds
    #[serde(default = "default_max_execution_time")]
    pub max_execution_time: u64,
    /// Maximum tool calls per task
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: usize,
    /// Maximum LLM calls per task
    #[serde(default = "default_max_llm_calls")]
    pub max_llm_calls: usize,
    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,
    /// Timeout configuration
    #[serde(default)]
    pub timeout: TimeoutConfig,
    /// Streaming configuration
    #[serde(default)]
    pub streaming: StreamingConfig,
}

/// Skill engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillEngineConfig {
    /// Skill directories
    #[serde(default)]
    pub directories: Vec<String>,
    /// Execution configuration
    #[serde(default)]
    pub execution: SkillExecutionConfig,
    /// Trigger configuration
    #[serde(default)]
    pub triggers: TriggerConfig,
    /// LLM parsing configuration
    #[serde(default)]
    pub llm_parsing: LlmParsingConfig,
}

/// Task manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskManagerConfig {
    /// Maximum parallel tasks
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
    /// Default timeout in seconds
    #[serde(default = "default_task_timeout")]
    pub default_timeout: u64,
    /// Check interval in seconds
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,
    /// Storage configuration
    #[serde(default)]
    pub storage: TaskStorageConfig,
    /// DAG configuration
    #[serde(default)]
    pub dag: DagConfig,
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfig {
    /// Workflow directories
    #[serde(default)]
    pub directories: Vec<String>,
    /// Execution configuration
    #[serde(default)]
    pub execution: WorkflowExecutionConfig,
    /// Versioning configuration
    #[serde(default)]
    pub versioning: VersioningConfig,
    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,
}

// ========== Supporting Types ==========

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: usize,
    /// Initial delay in milliseconds
    #[serde(default = "default_retry_delay")]
    pub delay: u64,
    /// Backoff strategy
    #[serde(default = "default_backoff")]
    pub backoff: String,
    /// Retryable error types
    #[serde(default)]
    pub retryable_errors: Vec<String>,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutConfig {
    /// LLM call timeout in seconds
    #[serde(default = "default_llm_call_timeout")]
    pub llm_call: u64,
    /// Tool call timeout in seconds
    #[serde(default = "default_tool_call_timeout")]
    pub tool_call: u64,
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingConfig {
    /// Enable streaming output
    #[serde(default = "default_streaming_enabled")]
    pub enabled: bool,
    /// Chunk size in tokens
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
}

/// Skill execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillExecutionConfig {
    /// Maximum steps per skill
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    /// Timeout in seconds
    #[serde(default = "default_skill_timeout")]
    pub timeout: u64,
    /// Enforce timeout
    #[serde(default = "default_enforce_timeout")]
    pub enforce_timeout: bool,
    /// Enforce max steps
    #[serde(default = "default_enforce_max_steps")]
    pub enforce_max_steps: bool,
}

/// Trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerConfig {
    /// Debounce time in milliseconds
    #[serde(default = "default_debounce")]
    pub debounce: u64,
    /// Maximum queue size
    #[serde(default = "default_max_queue_size")]
    pub max_queue_size: usize,
}

/// LLM parsing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmParsingConfig {
    /// Retry attempts
    #[serde(default = "default_parsing_retry")]
    pub retry: usize,
    /// Enable validation
    #[serde(default = "default_validation_enabled")]
    pub validation_enabled: bool,
}

/// Task storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStorageConfig {
    /// Persist task results
    #[serde(default = "default_persist_results")]
    pub persist_results: bool,
    /// Retention days
    #[serde(default = "default_retention_days")]
    pub retention_days: usize,
}

/// DAG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DagConfig {
    /// Maximum tasks in DAG
    #[serde(default = "default_max_dag_tasks")]
    pub max_tasks: usize,
    /// Maximum DAG depth
    #[serde(default = "default_max_dag_depth")]
    pub max_depth: usize,
}

/// Workflow execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowExecutionConfig {
    /// Default execution mode
    #[serde(default = "default_workflow_mode")]
    pub default_mode: String,
    /// Timeout in seconds
    #[serde(default = "default_workflow_timeout")]
    pub timeout: u64,
}

/// Versioning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersioningConfig {
    /// Enable versioning
    #[serde(default = "default_versioning_enabled")]
    pub enabled: bool,
    /// Enable git tracking
    #[serde(default = "default_git_tracking")]
    pub git_tracking: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// Enable cache
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,
    /// TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub ttl: u64,
}

// ========== Default Functions ==========

fn default_max_tasks() -> usize {
    10
}

fn default_task_timeout() -> u64 {
    300
}

fn default_max_execution_time() -> u64 {
    300
}

fn default_max_tool_calls() -> usize {
    50
}

fn default_max_llm_calls() -> usize {
    20
}

fn default_max_attempts() -> usize {
    3
}

fn default_retry_delay() -> u64 {
    1000
}

fn default_backoff() -> String {
    "exponential".to_string()
}

fn default_llm_call_timeout() -> u64 {
    60
}

fn default_tool_call_timeout() -> u64 {
    30
}

fn default_streaming_enabled() -> bool {
    true
}

fn default_chunk_size() -> usize {
    100
}

fn default_max_steps() -> usize {
    100
}

fn default_skill_timeout() -> u64 {
    600
}

fn default_enforce_timeout() -> bool {
    true
}

fn default_enforce_max_steps() -> bool {
    true
}

fn default_debounce() -> u64 {
    500
}

fn default_max_queue_size() -> usize {
    1000
}

fn default_parsing_retry() -> usize {
    3
}

fn default_validation_enabled() -> bool {
    true
}

fn default_max_parallel() -> usize {
    10
}

fn default_check_interval() -> u64 {
    5
}

fn default_persist_results() -> bool {
    true
}

fn default_retention_days() -> usize {
    30
}

fn default_max_dag_tasks() -> usize {
    1000
}

fn default_max_dag_depth() -> usize {
    50
}

fn default_workflow_mode() -> String {
    "background".to_string()
}

fn default_workflow_timeout() -> u64 {
    604800 // 7 days
}

fn default_versioning_enabled() -> bool {
    true
}

fn default_git_tracking() -> bool {
    true
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_ttl() -> u64 {
    3600
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            delay: default_retry_delay(),
            backoff: default_backoff(),
            retryable_errors: vec!["rate_limit".to_string(), "timeout".to_string(), "connection_error".to_string()],
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            llm_call: default_llm_call_timeout(),
            tool_call: default_tool_call_timeout(),
        }
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: default_streaming_enabled(),
            chunk_size: default_chunk_size(),
        }
    }
}

impl Default for SkillExecutionConfig {
    fn default() -> Self {
        Self {
            max_steps: default_max_steps(),
            timeout: default_skill_timeout(),
            enforce_timeout: default_enforce_timeout(),
            enforce_max_steps: default_enforce_max_steps(),
        }
    }
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            debounce: default_debounce(),
            max_queue_size: default_max_queue_size(),
        }
    }
}

impl Default for LlmParsingConfig {
    fn default() -> Self {
        Self {
            retry: default_parsing_retry(),
            validation_enabled: default_validation_enabled(),
        }
    }
}

impl Default for TaskStorageConfig {
    fn default() -> Self {
        Self {
            persist_results: default_persist_results(),
            retention_days: default_retention_days(),
        }
    }
}

impl Default for DagConfig {
    fn default() -> Self {
        Self {
            max_tasks: default_max_dag_tasks(),
            max_depth: default_max_dag_depth(),
        }
    }
}

impl Default for WorkflowExecutionConfig {
    fn default() -> Self {
        Self {
            default_mode: default_workflow_mode(),
            timeout: default_workflow_timeout(),
        }
    }
}

impl Default for VersioningConfig {
    fn default() -> Self {
        Self {
            enabled: default_versioning_enabled(),
            git_tracking: default_git_tracking(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_cache_enabled(),
            ttl: default_cache_ttl(),
        }
    }
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            max_execution_time: default_max_execution_time(),
            max_tool_calls: default_max_tool_calls(),
            max_llm_calls: default_max_llm_calls(),
            retry: RetryConfig::default(),
            timeout: TimeoutConfig::default(),
            streaming: StreamingConfig::default(),
        }
    }
}

impl Default for SkillEngineConfig {
    fn default() -> Self {
        Self {
            directories: vec!["./skills".to_string(), "~/.knight-agent/skills".to_string()],
            execution: SkillExecutionConfig::default(),
            triggers: TriggerConfig::default(),
            llm_parsing: LlmParsingConfig::default(),
        }
    }
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            max_parallel: default_max_parallel(),
            default_timeout: default_task_timeout(),
            check_interval: default_check_interval(),
            retry: RetryConfig::default(),
            storage: TaskStorageConfig::default(),
            dag: DagConfig::default(),
        }
    }
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            directories: vec!["./workflows".to_string(), "~/.knight-agent/workflows".to_string()],
            execution: WorkflowExecutionConfig::default(),
            versioning: VersioningConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

/// Logging configuration (config/logging.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Maximum log file size in MB
    #[serde(default = "default_max_log_size")]
    pub max_file_size_mb: u64,
    /// Maximum number of log files to keep
    #[serde(default = "default_max_log_files")]
    pub max_files: usize,
    /// Enable console output
    #[serde(default = "default_console_output")]
    pub console_output: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_log_size() -> u64 {
    10
}

fn default_max_log_files() -> usize {
    5
}

fn default_console_output() -> bool {
    true
}

/// Monitoring configuration (config/monitoring.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoringConfig {
    /// Enable monitoring
    #[serde(default = "default_monitoring")]
    pub enabled: bool,
    /// Metrics collection interval in seconds
    #[serde(default = "default_metrics_interval")]
    pub metrics_interval_secs: u64,
    /// Health check interval in seconds
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
}

fn default_monitoring() -> bool {
    false
}

fn default_metrics_interval() -> u64 {
    60
}

fn default_health_check_interval() -> u64 {
    30
}

/// Context compressor configuration (config/compressor.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressorConfig {
    /// Enable compression
    #[serde(default = "default_compression")]
    pub enabled: bool,
    /// Compression threshold in tokens
    #[serde(default = "default_threshold")]
    pub threshold_tokens: usize,
    /// Target compression ratio (0.0-1.0)
    #[serde(default = "default_compression_ratio")]
    pub target_ratio: f64,
    /// Compression strategy
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

fn default_compression() -> bool {
    true
}

fn default_threshold() -> usize {
    30000
}

fn default_compression_ratio() -> f64 {
    0.5
}

fn default_strategy() -> String {
    "semantic".to_string()
}

impl Default for KnightConfig {
    fn default() -> Self {
        Self {
            llm: None,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            max_file_size_mb: default_max_log_size(),
            max_files: default_max_log_files(),
            console_output: default_console_output(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: default_monitoring(),
            metrics_interval_secs: default_metrics_interval(),
            health_check_interval_secs: default_health_check_interval(),
        }
    }
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            enabled: default_compression(),
            threshold_tokens: default_threshold(),
            target_ratio: default_compression_ratio(),
            strategy: default_strategy(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_path: None,
            max_db_size_mb: default_max_db_size(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            sandbox_enabled: default_sandbox(),
            allowed_operations: Vec::new(),
            blocked_operations: Vec::new(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            default_variant: None,
            max_concurrent_tasks: default_max_tasks(),
            task_timeout_secs: default_task_timeout(),
            runtime: AgentRuntimeConfig::default(),
            skill: SkillEngineConfig::default(),
            task: TaskManagerConfig::default(),
            workflow: WorkflowConfig::default(),
        }
    }
}
