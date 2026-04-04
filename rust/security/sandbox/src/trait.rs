use crate::error::{SandboxError, SandboxResult};
use crate::types::{
    SandboxConfig, SandboxInfo, SandboxStatus, FileAction, AccessCheckResult,
    ResourceUsage, ResourceLimits, Violation
};

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
