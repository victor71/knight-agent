//! Bootstrap type definitions

use serde::{Deserialize, Serialize};

/// Bootstrap stage enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BootstrapStage {
    Stage1Infrastructure = 1,
    Stage2SecurityAndStorage = 2,
    Stage3BasicServicesAndEvent = 3,
    Stage4CoreEngineLayer = 4,
    Stage5AgentLayer = 5,
    Stage6Report = 6,
    Stage7ContextCompression = 7,
    Stage8SecurityLayer = 8,
}

impl BootstrapStage {
    /// Get stage number
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get stage name
    pub fn name(&self) -> &'static str {
        match self {
            BootstrapStage::Stage1Infrastructure => "Infrastructure",
            BootstrapStage::Stage2SecurityAndStorage => "SecurityAndStorage",
            BootstrapStage::Stage3BasicServicesAndEvent => "BasicServicesAndEvent",
            BootstrapStage::Stage4CoreEngineLayer => "CoreEngineLayer",
            BootstrapStage::Stage5AgentLayer => "AgentLayer",
            BootstrapStage::Stage6Report => "Report",
            BootstrapStage::Stage7ContextCompression => "ContextCompression",
            BootstrapStage::Stage8SecurityLayer => "SecurityLayer",
        }
    }

    /// Get all stages in order
    pub fn all() -> Vec<BootstrapStage> {
        vec![
            BootstrapStage::Stage1Infrastructure,
            BootstrapStage::Stage2SecurityAndStorage,
            BootstrapStage::Stage3BasicServicesAndEvent,
            BootstrapStage::Stage4CoreEngineLayer,
            BootstrapStage::Stage5AgentLayer,
            BootstrapStage::Stage6Report,
            BootstrapStage::Stage7ContextCompression,
            BootstrapStage::Stage8SecurityLayer,
        ]
    }

    /// Get modules for this stage
    pub fn modules(&self) -> Vec<&'static str> {
        match self {
            BootstrapStage::Stage1Infrastructure => vec!["logging-system"],
            BootstrapStage::Stage2SecurityAndStorage => vec!["security-manager", "storage-service"],
            BootstrapStage::Stage3BasicServicesAndEvent => vec!["llm-provider", "tool-system", "event-loop", "timer-system"],
            BootstrapStage::Stage4CoreEngineLayer => vec!["hook-engine", "session-manager", "router", "monitor"],
            BootstrapStage::Stage5AgentLayer => vec![
                "agent-variants",
                "agent-runtime",
                "external-agent",
                "skill-engine",
                "orchestrator",
                "task-manager",
                "command",
                "workflows-directory",
            ],
            BootstrapStage::Stage6Report => vec!["report-skill"],
            BootstrapStage::Stage7ContextCompression => vec!["context-compressor"],
            BootstrapStage::Stage8SecurityLayer => vec!["sandbox", "ipc-contract"],
        }
    }
}

impl std::fmt::Display for BootstrapStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stage {}: {}", self.as_u8(), self.name())
    }
}

/// Module status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub name: String,
    pub initialized: bool,
    pub healthy: bool,
    pub stage: u8,
    pub error: Option<String>,
}

impl ModuleStatus {
    /// Create new module status
    pub fn new(name: String, stage: BootstrapStage) -> Self {
        Self {
            name,
            initialized: false,
            healthy: false,
            stage: stage.as_u8(),
            error: None,
        }
    }

    /// Mark as initialized
    pub fn initialized(mut self) -> Self {
        self.initialized = true;
        self
    }

    /// Mark as healthy
    pub fn healthy(mut self) -> Self {
        self.healthy = true;
        self
    }

    /// Mark with error
    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }
}

/// System status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub stage: u8,
    pub initialized: bool,
    pub ready: bool,
    pub module_count: usize,
    pub initialized_count: usize,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub timestamp: i64,
    pub details: Vec<ModuleHealth>,
}

/// Module health info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleHealth {
    pub module: String,
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

/// Version info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub git_commit: Option<String>,
    pub build_time: Option<String>,
}

/// Bootstrap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub workspace: String,
    pub config_path: Option<String>,
    pub parallel_init: bool,
    pub init_timeout_ms: u64,
    pub retry_on_failure: bool,
    pub max_retries: usize,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            workspace: ".".to_string(),
            config_path: None,
            parallel_init: false,
            init_timeout_ms: 60000,
            retry_on_failure: true,
            max_retries: 3,
        }
    }
}
