//! Bootstrap system implementation

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{BootstrapError, BootstrapResult};
use crate::types::{
    BootstrapConfig, BootstrapMode, BootstrapStage, HealthCheckResult, ModuleHealth, ModuleStatus,
    SystemStatus, VersionInfo,
};

/// Knight Agent System - manages the 8-stage initialization
#[derive(Clone)]
pub struct KnightAgentSystem {
    config: BootstrapConfig,
    stage: Arc<RwLock<BootstrapStage>>,
    initialized: Arc<RwLock<bool>>,
    modules: Arc<RwLock<HashMap<String, ModuleStatus>>>,
    start_time: Arc<RwLock<Option<i64>>>,
}

impl KnightAgentSystem {
    /// Create a new system instance
    pub fn new() -> Self {
        Self::with_config(BootstrapConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: BootstrapConfig) -> Self {
        Self {
            config,
            stage: Arc::new(RwLock::new(BootstrapStage::Stage1Infrastructure)),
            initialized: Arc::new(RwLock::new(false)),
            modules: Arc::new(RwLock::new(HashMap::new())),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Get config
    pub fn config(&self) -> &BootstrapConfig {
        &self.config
    }

    /// Bootstrap the system through all 8 stages
    pub async fn bootstrap(&self) -> BootstrapResult<()> {
        if *self.initialized.read().await {
            return Err(BootstrapError::AlreadyInitialized);
        }

        *self.start_time.write().await = Some(chrono::Utc::now().timestamp_millis());

        let mode = self.config.mode;
        tracing::info!("Bootstrapping in {:?} mode", mode);

        for stage in BootstrapStage::all() {
            self.initialize_stage(stage, mode).await?;
        }

        *self.initialized.write().await = true;
        tracing::info!("Knight Agent System fully initialized in {:?} mode", mode);
        Ok(())
    }

    /// Initialize a specific stage
    async fn initialize_stage(&self, stage: BootstrapStage, mode: BootstrapMode) -> BootstrapResult<()> {
        tracing::info!("Initializing: {} (mode: {:?})", stage, mode);
        *self.stage.write().await = stage;

        let modules = stage.modules(mode);
        for module_name in modules {
            self.initialize_module(module_name, stage).await?;
        }

        tracing::info!("Completed: {}", stage);
        Ok(())
    }

    /// Initialize a single module
    async fn initialize_module(
        &self,
        name: &str,
        stage: BootstrapStage,
    ) -> BootstrapResult<()> {
        tracing::debug!("Initializing module: {} (Stage {})", name, stage.as_u8());

        // Record module status
        let mut status = ModuleStatus::new(name.to_string(), stage);

        // Simulate module initialization
        // In production, this would call the actual module's initialize method
        tokio::task::yield_now().await;

        // Mark as initialized and healthy
        status = status.initialized().healthy();

        self.modules.write().await.insert(name.to_string(), status);

        tracing::debug!("Module initialized: {}", name);
        Ok(())
    }

    /// Stop the system
    pub async fn stop(&self, graceful: bool, _timeout_ms: u64) -> BootstrapResult<bool> {
        tracing::info!("Stopping Knight Agent System (graceful: {})", graceful);

        if graceful {
            // Wait for ongoing operations to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Clear module status
        self.modules.write().await.clear();
        *self.initialized.write().await = false;
        *self.stage.write().await = BootstrapStage::Stage1Infrastructure;

        tracing::info!("Knight Agent System stopped");
        Ok(true)
    }

    /// Get current status
    pub async fn status(&self) -> SystemStatus {
        let stage = *self.stage.read().await;
        let initialized = *self.initialized.read().await;
        let modules = self.modules.read().await;

        let module_count = modules.len();
        let initialized_count = modules.values().filter(|m| m.initialized).count();

        SystemStatus {
            stage: stage.as_u8(),
            initialized,
            ready: initialized && module_count == 23, // Total module count
            module_count,
            initialized_count,
        }
    }

    /// Check if system is ready
    pub async fn is_ready(&self) -> bool {
        self.status().await.ready
    }

    /// Get all module statuses
    pub async fn module_statuses(&self) -> Vec<ModuleStatus> {
        self.modules.read().await.values().cloned().collect()
    }

    /// Get status of a specific module
    pub async fn module_status(&self, name: &str) -> Option<ModuleStatus> {
        self.modules.read().await.get(name).cloned()
    }

    /// Perform health check
    pub async fn health_check(&self, detailed: bool) -> BootstrapResult<HealthCheckResult> {
        let modules = self.modules.read().await;
        let mut all_healthy = true;
        let mut details = Vec::new();

        for (name, status) in modules.iter() {
            let healthy = status.healthy;
            if !healthy {
                all_healthy = false;
            }

            if detailed {
                details.push(ModuleHealth {
                    module: name.clone(),
                    healthy,
                    latency_ms: Some(0), // Would measure actual latency
                    message: if healthy {
                        None
                    } else {
                        Some("Module not healthy".to_string())
                    },
                });
            }
        }

        Ok(HealthCheckResult {
            healthy: all_healthy,
            timestamp: chrono::Utc::now().timestamp_millis(),
            details,
        })
    }

    /// Restart the system
    pub async fn restart(&self, graceful: bool) -> BootstrapResult<bool> {
        tracing::info!("Restarting Knight Agent System");

        self.stop(graceful, 5000).await?;
        self.bootstrap().await?;

        tracing::info!("Knight Agent System restarted");
        Ok(true)
    }

    /// Get version info
    pub fn version(&self) -> VersionInfo {
        VersionInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: option_env!("GIT_COMMIT").map(|s| s.to_string()),
            build_time: option_env!("BUILD_TIME").map(|s| s.to_string()),
        }
    }

    /// Get current stage
    pub async fn stage(&self) -> BootstrapStage {
        *self.stage.read().await
    }

    /// Check if initialized
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }
}

impl Default for KnightAgentSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// System handle trait for external access
#[async_trait::async_trait]
pub trait SystemHandle: Send + Sync {
    fn name(&self) -> &str;
    async fn is_initialized(&self) -> bool;
    async fn get_stage(&self) -> BootstrapStage;
}

/// System handle implementation
#[derive(Clone)]
pub struct SystemHandleImpl {
    system: KnightAgentSystem,
}

impl SystemHandleImpl {
    pub fn new(system: KnightAgentSystem) -> Self {
        Self { system }
    }
}

#[async_trait::async_trait]
impl SystemHandle for SystemHandleImpl {
    fn name(&self) -> &str {
        "knight-agent"
    }

    async fn is_initialized(&self) -> bool {
        self.system.is_initialized().await
    }

    async fn get_stage(&self) -> BootstrapStage {
        self.system.stage().await
    }
}
