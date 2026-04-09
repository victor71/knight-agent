//! Knight Agent Configuration Library
//!
//! This library provides centralized configuration management for Knight Agent,
//! supporting hot-reloadable JSON configuration files.
//!
//! # Configuration Structure
//!
//! ```text
//! ~/.knight-agent/
//! ├── knight.json          # Main configuration (LLM, storage, security, agent)
//! └── config/              # System configurations
//!     ├── logging.json     # Logging configuration
//!     ├── monitoring.json  # Monitoring configuration
//!     └── compressor.json  # Context compressor configuration
//! ```
//!
//! # Example
//!
//! ```no_run
//! use configuration::ConfigLoader;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config_dir = dirs::home_dir()
//!         .unwrap()
//!         .join(".knight-agent");
//!
//!     let loader = ConfigLoader::new(config_dir).await?;
//!
//!     // Get LLM configuration
//!     let llm_config = loader.get_llm_config();
//!
//!     // Subscribe to config changes
//!     let mut rx = loader.subscribe();
//!     tokio::spawn(async move {
//!         while let Ok(change) = rx.recv().await {
//!             match change {
//!                 configuration::ConfigChangeEvent::MainConfigChanged(config) => {
//!                     // Handle main config change
//!                 }
//!                 configuration::ConfigChangeEvent::SystemConfigChanged { name, config } => {
//!                     // Handle system config change
//!                 }
//!             }
//!         }
//!     });
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod loader;
pub mod types;

pub use error::{ConfigError, ConfigResult};
pub use loader::{
    ConfigChangeEvent, ConfigLoader, SystemConfig,
    init_global_config, get_global_config, get_llm_config, subscribe_config_changes
};
pub use types::{
    AgentConfig, CompressorConfig, CoreConfig, InfrastructureConfig, KnightConfig,
    LlmConfig, LlmModelConfig, LlmPricing, LlmProviderConfig, LoggingConfig, MonitoringConfig,
    SecurityConfig, ServicesConfig, StorageConfig, ToolsConfig,
    // Config sub-types
    AgentRuntimeConfig, SkillEngineConfig, TaskManagerConfig, WorkflowConfig,
    CommandConfig, CliConfig, EventLoopConfig, HooksConfig, OrchestratorConfig,
    RouterConfig, SessionConfig, BootstrapConfig,
    McpConfig, ReportConfig, TimerConfig,
    ToolsBuiltinConfig, ToolsCustomConfig, ToolsMcpConfig,
    IpcConfig,
    CliConnectionConfig, CliOutputConfig,
};

/// Get the default Knight Agent configuration directory
///
/// Returns `~/.knight-agent` on all platforms
pub fn default_config_dir() -> Option<std::path::PathBuf> {
    ::dirs::home_dir().map(|home: std::path::PathBuf| home.join(".knight-agent"))
}
