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
    get_global_config, get_llm_config, init_global_config, subscribe_config_changes,
    ConfigChangeEvent, ConfigLoader, SystemConfig,
};
pub use types::{
    AgentConfig,
    // Config sub-types
    AgentRuntimeConfig,
    BootstrapConfig,
    CliConfig,
    CliConnectionConfig,
    CliOutputConfig,
    CommandConfig,
    CompressorConfig,
    CoreConfig,
    EventLoopConfig,
    HooksConfig,
    InfrastructureConfig,
    IpcConfig,
    KnightConfig,
    LlmConfig,
    LlmModelConfig,
    LlmPricing,
    LlmProviderConfig,
    LoggingConfig,
    McpConfig,
    MonitoringConfig,
    OrchestratorConfig,
    ReportConfig,
    RouterConfig,
    SecurityConfig,
    ServicesConfig,
    SessionConfig,
    SkillEngineConfig,
    StorageConfig,
    TaskManagerConfig,
    TimerConfig,
    ToolsBuiltinConfig,
    ToolsConfig,
    ToolsCustomConfig,
    ToolsMcpConfig,
    WorkflowConfig,
};

/// Get the default Knight Agent configuration directory
///
/// Returns `~/.knight-agent` on all platforms
pub fn default_config_dir() -> Option<std::path::PathBuf> {
    ::dirs::home_dir().map(|home: std::path::PathBuf| home.join(".knight-agent"))
}
