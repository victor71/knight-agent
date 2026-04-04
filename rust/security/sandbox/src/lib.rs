//! Sandbox Module
//!
//! Provides resource isolation and security boundaries for agent operations.
//!
//! Design Reference: docs/03-module-design/security/sandbox.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

pub type SandboxResult<T> = Result<T, SandboxError>;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Sandbox not initialized")]
    NotInitialized,
    #[error("Sandbox creation failed: {0}")]
    CreationFailed(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("Sandbox not found: {0}")]
    SandboxNotFound(String),
    #[error("Access denied: {0}")]
    AccessDenied(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Violation reported: {0}")]
    Violation(String),
}

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

/// Violation action
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ViolationAction {
    Log,
    Warn,
    Block,
    Terminate,
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

/// Sandbox trait - provides security boundaries for agent operations
#[allow(async_fn_in_trait)]
pub trait Sandbox: Send + Sync {
    fn new(config: SandboxConfig) -> Result<Self, SandboxError>
    where
        Self: Sized;

    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;

    // Sandbox management
    async fn create_sandbox(&self, config: SandboxConfig) -> SandboxResult<String>;
    async fn destroy_sandbox(&self, sandbox_id: &str) -> SandboxResult<()>;
    async fn get_sandbox(&self, sandbox_id: &str) -> SandboxResult<Option<SandboxInfo>>;
    async fn list_sandboxes(&self, status: Option<SandboxStatus>) -> SandboxResult<Vec<SandboxInfo>>;

    // Access control
    async fn check_file_access(
        &self,
        sandbox_id: &str,
        path: &str,
        action: FileAction,
    ) -> SandboxResult<AccessCheckResult>;

    async fn check_command_access(
        &self,
        sandbox_id: &str,
        command: &str,
        args: &[String],
    ) -> SandboxResult<AccessCheckResult>;

    async fn check_network_access(
        &self,
        sandbox_id: &str,
        host: &str,
        port: u16,
    ) -> SandboxResult<AccessCheckResult>;

    // Resource monitoring
    async fn get_resource_usage(&self, sandbox_id: &str) -> SandboxResult<ResourceUsage>;
    async fn get_resource_limits(&self, sandbox_id: &str) -> SandboxResult<ResourceLimits>;
    async fn set_resource_limits(&self, sandbox_id: &str, limits: ResourceLimits) -> SandboxResult<()>;

    // Violation handling
    async fn get_violations(
        &self,
        sandbox_id: &str,
        time_range: Option<(String, String)>,
    ) -> SandboxResult<Vec<Violation>>;
    async fn report_violation(&self, sandbox_id: &str, violation: Violation) -> SandboxResult<String>;

    // Configuration
    async fn get_sandbox_config(&self, sandbox_id: &str) -> SandboxResult<SandboxConfig>;
    async fn update_sandbox_config(&self, sandbox_id: &str, config: SandboxConfig) -> SandboxResult<()>;
}

/// Permission checker helper
pub struct PermissionChecker {
    workspace: String,
    filesystem: FilesystemSandbox,
    command: CommandSandbox,
    network: NetworkSandbox,
}

impl PermissionChecker {
    pub fn new(config: &SandboxConfig) -> Self {
        Self {
            workspace: config.workspace.clone(),
            filesystem: config.filesystem.clone(),
            command: config.command.clone(),
            network: config.network.clone(),
        }
    }

    /// Check file access permission
    pub fn check_file_access(&self, path: &str, action: FileAction) -> AccessCheckResult {
        // Normalize path to absolute
        let abs_path = if Path::new(path).is_absolute() {
            path.to_string()
        } else {
            format!("{}/{}", self.workspace, path)
        };

        // Check denied patterns first
        for pattern in &self.filesystem.denied_patterns {
            if glob_match(pattern, &abs_path) {
                return AccessCheckResult::denied(format!("Path matches denied pattern: {}", pattern));
            }
        }

        // Check if path is in allowed paths
        let allowed = self.filesystem.allowed_paths.iter().any(|p| {
            glob_match(p, &abs_path) || abs_path.starts_with(p.trim_end_matches("**"))
        });

        if !allowed {
            return AccessCheckResult::denied("Path not in allowed paths");
        }

        // Check read-only for write/delete actions
        if matches!(action, FileAction::Write | FileAction::Delete) {
            for ro_path in &self.filesystem.read_only {
                if glob_match(ro_path, &abs_path) {
                    return AccessCheckResult::denied(format!("Path is read-only: {}", ro_path));
                }
            }
        }

        AccessCheckResult::allowed()
    }

    /// Check command execution permission
    pub fn check_command(&self, command: &str, _args: &[String]) -> AccessCheckResult {
        // Check denied commands first
        for denied in &self.command.denied_commands {
            if command.contains(denied) || denied == command {
                return AccessCheckResult::denied(format!("Command in denied list: {}", denied));
            }
        }

        // If whitelist is non-empty, command must be in it
        if !self.command.allowed_commands.is_empty() {
            let allowed = self.command.allowed_commands.iter().any(|c| command.starts_with(c));
            if !allowed {
                return AccessCheckResult::denied("Command not in allowed list");
            }
        }

        AccessCheckResult::allowed()
    }

    /// Check network access permission
    pub fn check_network(&self, host: &str, port: u16) -> AccessCheckResult {
        if !self.network.enabled {
            return AccessCheckResult::denied("Network access is disabled");
        }

        // Check denied hosts
        for denied in &self.network.denied_hosts {
            if host.contains(denied) || denied == host {
                return AccessCheckResult::denied(format!("Host in denied list: {}", denied));
            }
        }

        // If whitelist is non-empty, host must be in it
        if !self.network.allowed_hosts.is_empty() {
            let allowed = self.network.allowed_hosts.iter().any(|h| host.contains(h) || h == host);
            if !allowed {
                return AccessCheckResult::denied("Host not in allowed list");
            }
        }

        // Check port ranges
        if !self.network.allowed_ports.is_empty() {
            let port_allowed = self.network.allowed_ports.iter().any(|r| port >= r.start && port <= r.end);
            if !port_allowed {
                return AccessCheckResult::denied(format!("Port {} not in allowed ranges", port));
            }
        }

        AccessCheckResult::allowed()
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim();
    let path = path.trim();

    // Match everything
    if pattern == "**/*" || pattern == "*" {
        return true;
    }

    // Match .env file anywhere
    if pattern == "**/.env" {
        return path == ".env" || path.ends_with("/.env") || path.contains("/.env/");
    }

    // Match .git directory anywhere
    if pattern == "**/.git/**" {
        return path.starts_with(".git/") ||
               path.contains("/.git/") ||
               path == ".git" ||
               path.starts_with(".git") ||
               path.ends_with("/.git") ||
               path.contains("/.git/");
    }

    // Match **/*.rs - any .rs file anywhere
    if pattern == "**/*.rs" {
        return path.ends_with(".rs");
    }

    // Match /foo/** pattern - /foo and anything under it
    if pattern.ends_with("/**") {
        let base = &pattern[..pattern.len() - 3];
        return path == base ||
               path.starts_with(&format!("{}/", base)) ||
               path.starts_with(base) ||
               path.contains(&format!("{}/", base));
    }

    // Handle * glob (matches any filename, not across /)
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        for part in &parts {
            if part.is_empty() {
                continue;
            }
            if let Some(idx) = path[pos..].find(part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }
        return pos == path.len();
    }

    // Exact match or prefix match
    pattern == path || path.starts_with(&format!("{}/", pattern))
}

pub struct SandboxImpl {
    sandboxes: Arc<RwLock<HashMap<String, SandboxInfo>>>,
    violations: Arc<RwLock<HashMap<String, Vec<Violation>>>>,
    initialized: Arc<RwLock<bool>>,
}

impl SandboxImpl {
    pub fn new() -> Self {
        Self {
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            violations: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    fn generate_id() -> String {
        format!("sandbox-{}", uuid::Uuid::new_v4())
    }

    fn now_iso() -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

impl Default for SandboxImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox for SandboxImpl {
    fn new(_config: SandboxConfig) -> Result<Self, SandboxError> {
        Ok(Self::new())
    }

    fn name(&self) -> &str {
        "sandbox"
    }

    fn is_initialized(&self) -> bool {
        // Use try_read to avoid blocking
        self.initialized.try_read().map(|g| *g).unwrap_or(false)
    }

    async fn create_sandbox(&self, config: SandboxConfig) -> SandboxResult<String> {
        let id = Self::generate_id();
        let info = SandboxInfo {
            id: id.clone(),
            level: config.level,
            status: SandboxStatus::Active,
            created_at: Self::now_iso(),
            config: config.clone(),
            usage: ResourceUsage::default(),
            violation_count: 0,
        };

        self.sandboxes.write().await.insert(id.clone(), info);
        self.violations.write().await.insert(id.clone(), Vec::new());

        tracing::info!("Created sandbox: {}", id);
        Ok(id)
    }

    async fn destroy_sandbox(&self, sandbox_id: &str) -> SandboxResult<()> {
        let mut sandboxes = self.sandboxes.write().await;
        if let Some(info) = sandboxes.get_mut(sandbox_id) {
            info.status = SandboxStatus::Terminated;
            tracing::info!("Destroyed sandbox: {}", sandbox_id);
            Ok(())
        } else {
            Err(SandboxError::SandboxNotFound(sandbox_id.to_string()))
        }
    }

    async fn get_sandbox(&self, sandbox_id: &str) -> SandboxResult<Option<SandboxInfo>> {
        let sandboxes = self.sandboxes.read().await;
        Ok(sandboxes.get(sandbox_id).cloned())
    }

    async fn list_sandboxes(&self, status: Option<SandboxStatus>) -> SandboxResult<Vec<SandboxInfo>> {
        let sandboxes = self.sandboxes.read().await;
        let mut result: Vec<_> = sandboxes.values().cloned().collect();

        if let Some(status_filter) = status {
            result.retain(|s| s.status == status_filter);
        }

        Ok(result)
    }

    async fn check_file_access(
        &self,
        sandbox_id: &str,
        path: &str,
        action: FileAction,
    ) -> SandboxResult<AccessCheckResult> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes.get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        let checker = PermissionChecker::new(&info.config);
        let result = checker.check_file_access(path, action);

        if !result.allowed {
            let violation = Violation {
                id: format!("vio-{}", uuid::Uuid::new_v4()),
                sandbox_id: sandbox_id.to_string(),
                timestamp: Self::now_iso(),
                violation_type: ViolationType::FileAccessDenied,
                severity: ViolationSeverity::Medium,
                description: format!("File access denied: {} for {:?}", path, action),
                details: HashMap::new(),
            };
            drop(sandboxes);
            self.report_violation(sandbox_id, violation).await?;
        }

        Ok(result)
    }

    async fn check_command_access(
        &self,
        sandbox_id: &str,
        command: &str,
        args: &[String],
    ) -> SandboxResult<AccessCheckResult> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes.get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        let checker = PermissionChecker::new(&info.config);
        let result = checker.check_command(command, args);

        if !result.allowed {
            let violation = Violation {
                id: format!("vio-{}", uuid::Uuid::new_v4()),
                sandbox_id: sandbox_id.to_string(),
                timestamp: Self::now_iso(),
                violation_type: ViolationType::CommandDenied,
                severity: ViolationSeverity::High,
                description: format!("Command denied: {}", command),
                details: HashMap::new(),
            };
            drop(sandboxes);
            self.report_violation(sandbox_id, violation).await?;
        }

        Ok(result)
    }

    async fn check_network_access(
        &self,
        sandbox_id: &str,
        host: &str,
        port: u16,
    ) -> SandboxResult<AccessCheckResult> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes.get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        let checker = PermissionChecker::new(&info.config);
        let result = checker.check_network(host, port);

        if !result.allowed {
            let violation = Violation {
                id: format!("vio-{}", uuid::Uuid::new_v4()),
                sandbox_id: sandbox_id.to_string(),
                timestamp: Self::now_iso(),
                violation_type: ViolationType::NetworkDenied,
                severity: ViolationSeverity::Medium,
                description: format!("Network access denied: {}:{}", host, port),
                details: HashMap::new(),
            };
            drop(sandboxes);
            self.report_violation(sandbox_id, violation).await?;
        }

        Ok(result)
    }

    async fn get_resource_usage(&self, sandbox_id: &str) -> SandboxResult<ResourceUsage> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes.get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        Ok(info.usage.clone())
    }

    async fn get_resource_limits(&self, sandbox_id: &str) -> SandboxResult<ResourceLimits> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes.get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        Ok(ResourceLimits {
            max_memory_mb: info.config.filesystem.max_total_size / (1024 * 1024),
            max_cpu_percent: 80.0,
            max_execution_time: info.config.command.max_execution_time,
            max_file_handles: 100,
        })
    }

    async fn set_resource_limits(&self, sandbox_id: &str, _limits: ResourceLimits) -> SandboxResult<()> {
        // In a real implementation, this would update the resource limits
        // For now, we just acknowledge the request
        let _ = sandbox_id;
        Ok(())
    }

    async fn get_violations(
        &self,
        sandbox_id: &str,
        _time_range: Option<(String, String)>,
    ) -> SandboxResult<Vec<Violation>> {
        let violations = self.violations.read().await;
        Ok(violations.get(sandbox_id).cloned().unwrap_or_default())
    }

    async fn report_violation(&self, sandbox_id: &str, violation: Violation) -> SandboxResult<String> {
        let vid = violation.id.clone();

        // Record the violation
        let mut violations = self.violations.write().await;
        violations
            .entry(sandbox_id.to_string())
            .or_insert_with(Vec::new)
            .push(violation.clone());

        // Update violation count in sandbox info
        let mut sandboxes = self.sandboxes.write().await;
        if let Some(info) = sandboxes.get_mut(sandbox_id) {
            info.violation_count += 1;
        }

        // Log based on violation action
        match violation.severity {
            ViolationSeverity::Low | ViolationSeverity::Medium => {
                tracing::warn!("Sandbox violation [{}]: {}", sandbox_id, violation.description);
            }
            ViolationSeverity::High | ViolationSeverity::Critical => {
                tracing::error!("Sandbox violation [{}]: {}", sandbox_id, violation.description);
            }
        }

        Ok(vid)
    }

    async fn get_sandbox_config(&self, sandbox_id: &str) -> SandboxResult<SandboxConfig> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes.get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        Ok(info.config.clone())
    }

    async fn update_sandbox_config(&self, sandbox_id: &str, config: SandboxConfig) -> SandboxResult<()> {
        let mut sandboxes = self.sandboxes.write().await;
        if let Some(info) = sandboxes.get_mut(sandbox_id) {
            info.config = config;
            Ok(())
        } else {
            Err(SandboxError::SandboxNotFound(sandbox_id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("**/*.rs", "foo/bar/baz.rs"));
        assert!(glob_match("**/.env", ".env"));
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("**/*", "anything/here.txt"));
        assert!(!glob_match("**/.git/**", "src/main.rs"));
        assert!(glob_match("**/.git/**", ".git/config"));
    }

    #[test]
    fn test_permission_checker_file() {
        let config = SandboxConfig::default();
        let checker = PermissionChecker::new(&config);

        // Basic access should be allowed
        let result = checker.check_file_access("/tmp/test.txt", FileAction::Read);
        assert!(result.allowed);
    }

    #[test]
    fn test_permission_checker_denied_path() {
        let mut config = SandboxConfig::default();
        config.filesystem.denied_patterns.push("**/.env".to_string());

        let checker = PermissionChecker::new(&config);
        let result = checker.check_file_access("/project/.env", FileAction::Read);
        assert!(!result.allowed);
    }

    #[test]
    fn test_permission_checker_readonly() {
        let mut config = SandboxConfig::default();
        config.filesystem.read_only.push("/protected/**".to_string());

        let checker = PermissionChecker::new(&config);
        let result = checker.check_file_access("/protected/file.txt", FileAction::Write);
        assert!(!result.allowed);
    }

    #[test]
    fn test_permission_checker_command() {
        let config = SandboxConfig::default();
        let checker = PermissionChecker::new(&config);

        // rm -rf / should be denied
        let result = checker.check_command("rm -rf /", &[]);
        assert!(!result.allowed);
    }

    #[test]
    fn test_permission_checker_network() {
        let config = SandboxConfig::default();
        let checker = PermissionChecker::new(&config);

        // Network enabled by default, should allow
        let result = checker.check_network("api.example.com", 443);
        assert!(result.allowed);
    }

    #[test]
    fn test_sandbox_impl_new() {
        let sandbox = SandboxImpl::new();
        assert_eq!(sandbox.name(), "sandbox");
        assert!(!sandbox.is_initialized());
    }

    #[tokio::test]
    async fn test_create_sandbox() {
        let sandbox = SandboxImpl::new();
        let config = SandboxConfig::default();

        let id = sandbox.create_sandbox(config).await.unwrap();
        assert!(!id.is_empty());

        let info = sandbox.get_sandbox(&id).await.unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().status, SandboxStatus::Active);
    }

    #[tokio::test]
    async fn test_destroy_sandbox() {
        let sandbox = SandboxImpl::new();
        let config = SandboxConfig::default();

        let id = sandbox.create_sandbox(config).await.unwrap();
        sandbox.destroy_sandbox(&id).await.unwrap();

        let info = sandbox.get_sandbox(&id).await.unwrap();
        assert_eq!(info.unwrap().status, SandboxStatus::Terminated);
    }

    #[tokio::test]
    async fn test_check_file_access() {
        let sandbox = SandboxImpl::new();
        let config = SandboxConfig::default();

        let id = sandbox.create_sandbox(config).await.unwrap();
        let result = sandbox.check_file_access(&id, "/tmp/test.txt", FileAction::Read).await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_check_command_access() {
        let sandbox = SandboxImpl::new();
        let config = SandboxConfig::default();

        let id = sandbox.create_sandbox(config).await.unwrap();

        // Safe command should be allowed
        let result = sandbox.check_command_access(&id, "git", &["status".to_string()]).await.unwrap();
        assert!(result.allowed);

        // Dangerous command should be denied
        let result = sandbox.check_command_access(&id, "rm -rf /", &[]).await.unwrap();
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_violation_reporting() {
        let sandbox = SandboxImpl::new();
        let config = SandboxConfig::default();

        let id = sandbox.create_sandbox(config).await.unwrap();
        let violation = Violation {
            id: "test-vio-1".to_string(),
            sandbox_id: id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            violation_type: ViolationType::FileAccessDenied,
            severity: ViolationSeverity::Medium,
            description: "Test violation".to_string(),
            details: HashMap::new(),
        };

        sandbox.report_violation(&id, violation).await.unwrap();

        let violations = sandbox.get_violations(&id, None).await.unwrap();
        assert_eq!(violations.len(), 1);
    }
}
