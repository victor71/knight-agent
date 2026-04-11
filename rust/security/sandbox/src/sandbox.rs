use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::checker::PermissionChecker;
use crate::error::{SandboxError, SandboxResult};
use crate::r#trait::Sandbox;
use crate::types::{
    AccessCheckResult, FileAction, ResourceLimits, ResourceUsage, SandboxConfig, SandboxInfo,
    SandboxStatus, Violation, ViolationSeverity, ViolationType,
};

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

    async fn list_sandboxes(
        &self,
        status: Option<SandboxStatus>,
    ) -> SandboxResult<Vec<SandboxInfo>> {
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
        let info = sandboxes
            .get(sandbox_id)
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
        let info = sandboxes
            .get(sandbox_id)
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
        let info = sandboxes
            .get(sandbox_id)
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
        let info = sandboxes
            .get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        Ok(info.usage.clone())
    }

    async fn get_resource_limits(&self, sandbox_id: &str) -> SandboxResult<ResourceLimits> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes
            .get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        Ok(ResourceLimits {
            max_memory_mb: info.config.filesystem.max_total_size / (1024 * 1024),
            max_cpu_percent: 80.0,
            max_execution_time: info.config.command.max_execution_time,
            max_file_handles: 100,
        })
    }

    async fn set_resource_limits(
        &self,
        sandbox_id: &str,
        _limits: ResourceLimits,
    ) -> SandboxResult<()> {
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

    async fn report_violation(
        &self,
        sandbox_id: &str,
        violation: Violation,
    ) -> SandboxResult<String> {
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
                tracing::warn!(
                    "Sandbox violation [{}]: {}",
                    sandbox_id,
                    violation.description
                );
            }
            ViolationSeverity::High | ViolationSeverity::Critical => {
                tracing::error!(
                    "Sandbox violation [{}]: {}",
                    sandbox_id,
                    violation.description
                );
            }
        }

        Ok(vid)
    }

    async fn get_sandbox_config(&self, sandbox_id: &str) -> SandboxResult<SandboxConfig> {
        let sandboxes = self.sandboxes.read().await;
        let info = sandboxes
            .get(sandbox_id)
            .ok_or_else(|| SandboxError::SandboxNotFound(sandbox_id.to_string()))?;

        Ok(info.config.clone())
    }

    async fn update_sandbox_config(
        &self,
        sandbox_id: &str,
        config: SandboxConfig,
    ) -> SandboxResult<()> {
        let mut sandboxes = self.sandboxes.write().await;
        if let Some(info) = sandboxes.get_mut(sandbox_id) {
            info.config = config;
            Ok(())
        } else {
            Err(SandboxError::SandboxNotFound(sandbox_id.to_string()))
        }
    }
}
