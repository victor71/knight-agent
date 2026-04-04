use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sandbox isolation level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SandboxLevel {
    None,   // No isolation - local development
    Basic,   // Basic isolation - general use
    Strict,  // Strict isolation - sensitive operations
    Full,    // Full isolation - untrusted code
}

impl Default for SandboxLevel {
    fn default() -> Self {
        SandboxLevel::Basic
    }
}

/// File system access action
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileAction {
    Read,
    Write,
    Delete,
    Execute,
}

/// Sandbox status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SandboxStatus {
    Active,
    Paused,
    Terminated,
}

/// Violation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    FileAccessDenied,
    CommandDenied,
    NetworkDenied,
    ResourceExceeded,
    MaliciousBehavior,
}

/// Violation severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: String,
    pub sandbox_id: String,
    pub timestamp: String,
    pub violation_type: ViolationType,
    pub severity: ViolationSeverity,
    pub description: String,
    pub details: HashMap<String, serde_json::Value>,
}

/// File system sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSandbox {
    pub allowed_paths: Vec<String>,
    pub denied_patterns: Vec<String>,
    pub read_only: Vec<String>,
    #[serde(default)]
    pub max_file_size: u64,
    #[serde(default)]
    pub max_total_size: u64,
}

impl Default for FilesystemSandbox {
    fn default() -> Self {
        Self {
            allowed_paths: vec!["**/*".to_string()],
            denied_patterns: vec![
                "**/.git/**".to_string(),
                "**/.env".to_string(),
            ],
            read_only: vec![],
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_total_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Command sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSandbox {
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    pub denied_commands: Vec<String>,
    #[serde(default)]
    pub max_execution_time: u64,
    #[serde(default)]
    pub max_concurrent: u64,
}

impl Default for CommandSandbox {
    fn default() -> Self {
        Self {
            allowed_commands: vec![],
            denied_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd".to_string(),
            ],
            max_execution_time: 300,
            max_concurrent: 5,
        }
    }
}

/// Network sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSandbox {
    #[serde(default = "default_network_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub denied_hosts: Vec<String>,
    #[serde(default)]
    pub allowed_ports: Vec<PortRange>,
    #[serde(default)]
    pub max_connections: u64,
    #[serde(default)]
    pub max_bandwidth: u64,
}

fn default_network_enabled() -> bool {
    true
}

impl Default for NetworkSandbox {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_hosts: vec![],
            denied_hosts: vec![],
            allowed_ports: vec![],
            max_connections: 10,
            max_bandwidth: 1024 * 1024 * 10, // 10MB/s
        }
    }
}

/// Port range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u64,
    #[serde(default = "default_max_cpu_percent")]
    pub max_cpu_percent: f64,
    #[serde(default)]
    pub max_execution_time: u64,
    #[serde(default)]
    pub max_file_handles: u64,
}

fn default_max_memory_mb() -> u64 {
    1024
}

fn default_max_cpu_percent() -> f64 {
    80.0
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,
            max_cpu_percent: 80.0,
            max_execution_time: 600,
            max_file_handles: 100,
        }
    }
}

/// Violation action
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ViolationAction {
    Log,
    Warn,
    Block,
    Terminate,
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub level: SandboxLevel,
    pub workspace: String,
    pub filesystem: FilesystemSandbox,
    pub command: CommandSandbox,
    pub network: NetworkSandbox,
    #[serde(default = "default_violation_action")]
    pub violation_action: ViolationAction,
    #[serde(default = "default_log_violations")]
    pub log_violations: bool,
}

fn default_violation_action() -> ViolationAction {
    ViolationAction::Warn
}

fn default_log_violations() -> bool {
    true
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            level: SandboxLevel::Basic,
            workspace: ".".to_string(),
            filesystem: FilesystemSandbox::default(),
            command: CommandSandbox::default(),
            network: NetworkSandbox::default(),
            violation_action: ViolationAction::Warn,
            log_violations: true,
        }
    }
}

/// Resource usage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    #[serde(default)]
    pub memory_mb: u64,
    #[serde(default)]
    pub cpu_percent: f64,
    #[serde(default)]
    pub execution_time: u64,
    #[serde(default)]
    pub file_handles: u64,
    #[serde(default)]
    pub network_connections: u64,
    #[serde(default)]
    pub disk_usage: u64,
}

/// Sandbox info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub id: String,
    pub level: SandboxLevel,
    pub status: SandboxStatus,
    pub created_at: String,
    pub config: SandboxConfig,
    #[serde(default)]
    pub usage: ResourceUsage,
    #[serde(default)]
    pub violation_count: u64,
}

/// Access check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessCheckResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl AccessCheckResult {
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    pub fn denied(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
        }
    }
}
