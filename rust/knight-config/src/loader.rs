//! Configuration loader with hot reload support

use crate::error::{ConfigError, ConfigResult};
use crate::types::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Configuration change event
#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    /// Main config changed
    MainConfigChanged(KnightConfig),
    /// System config changed
    SystemConfigChanged { name: String, config: SystemConfig },
}

/// System configuration variants (from config/*.yaml files)
#[derive(Debug, Clone)]
pub enum SystemConfig {
    /// Logging configuration (config/logging.yaml)
    Logging(LoggingConfig),
    /// Monitoring configuration (config/monitoring.yaml)
    Monitoring(MonitoringConfig),
    /// Compressor configuration (config/compressor.yaml)
    Compressor(CompressorConfig),
    /// Storage configuration (config/storage.yaml)
    Storage(StorageConfig),
    /// Security configuration (config/security.yaml)
    Security(SecurityConfig),
    /// Agent configuration (config/agent.yaml)
    Agent(AgentConfig),
}

/// Configuration loader
pub struct ConfigLoader {
    /// Config directory path
    config_dir: PathBuf,
    /// Main knight.json config
    main_config: Arc<RwLock<KnightConfig>>,
    /// System configs (logging, monitoring, etc.)
    system_configs: Arc<RwLock<HashMap<String, SystemConfig>>>,
    /// Config change event sender
    change_tx: broadcast::Sender<ConfigChangeEvent>,
    /// File watcher
    _watcher: RecommendedWatcher,
}

impl ConfigLoader {
    /// Create a new config loader
    pub async fn new(config_dir: PathBuf) -> ConfigResult<Self> {
        let (change_tx, _) = broadcast::channel(32);

        // Ensure config directory exists
        if !config_dir.exists() {
            tokio::fs::create_dir_all(&config_dir).await?;
        }

        // Create config subdirectory
        let system_config_dir = config_dir.join("config");
        if !system_config_dir.exists() {
            tokio::fs::create_dir_all(&system_config_dir).await?;
        }

        // Load initial configurations
        let main_config = Arc::new(RwLock::new(Self::load_main_config(&config_dir).await?));
        let system_configs = Arc::new(RwLock::new(Self::load_system_configs(&system_config_dir).await?));

        // Setup file watcher
        let watcher = Self::setup_watcher(
            config_dir.clone(),
            main_config.clone(),
            system_configs.clone(),
            change_tx.clone(),
        )?;

        info!("Config loader initialized: {}", config_dir.display());

        Ok(Self {
            config_dir,
            main_config,
            system_configs,
            change_tx,
            _watcher: watcher,
        })
    }

    /// Load main configuration from knight.json (LLM config only)
    async fn load_main_config(config_dir: &Path) -> ConfigResult<KnightConfig> {
        let config_path = config_dir.join("knight.json");

        if !config_path.exists() {
            info!("knight.json not found, creating default config");
            let default_config = KnightConfig::default();
            Self::save_main_config(config_dir, &default_config).await?;
            return Ok(default_config);
        }

        let content = tokio::fs::read_to_string(&config_path).await?;
        let config: KnightConfig = serde_json::from_str(&content)?;

        info!("Loaded main config from: {}", config_path.display());
        Ok(config)
    }

    /// Save main configuration
    async fn save_main_config(config_dir: &Path, config: &KnightConfig) -> ConfigResult<()> {
        let config_path = config_dir.join("knight.json");
        let content = serde_json::to_string_pretty(config)?;
        tokio::fs::write(&config_path, content).await?;
        info!("Saved main config to: {}", config_path.display());
        Ok(())
    }

    /// Load all system configurations from config/ directory (YAML format)
    async fn load_system_configs(config_dir: &Path) -> ConfigResult<HashMap<String, SystemConfig>> {
        let mut configs = HashMap::new();

        // Define system config files and their loaders
        let system_files: &[(&str, fn(&str) -> Result<SystemConfig, serde_yaml::Error>)] = &[
            ("agent", |content| {
                serde_yaml::from_str(content).map(SystemConfig::Agent)
            }),
            ("logging", |content| {
                serde_yaml::from_str(content).map(SystemConfig::Logging)
            }),
            ("monitoring", |content| {
                serde_yaml::from_str(content).map(SystemConfig::Monitoring)
            }),
            ("compressor", |content| {
                serde_yaml::from_str(content).map(SystemConfig::Compressor)
            }),
            ("storage", |content| {
                serde_yaml::from_str(content).map(SystemConfig::Storage)
            }),
            ("security", |content| {
                serde_yaml::from_str(content).map(SystemConfig::Security)
            }),
        ];

        let default_configs: &[(&str, SystemConfig)] = &[
            ("agent", SystemConfig::Agent(AgentConfig::default())),
            ("logging", SystemConfig::Logging(LoggingConfig::default())),
            ("monitoring", SystemConfig::Monitoring(MonitoringConfig::default())),
            ("compressor", SystemConfig::Compressor(CompressorConfig::default())),
            ("storage", SystemConfig::Storage(StorageConfig::default())),
            ("security", SystemConfig::Security(SecurityConfig::default())),
        ];

        for (name, loader) in system_files {
            let config_path = config_dir.join(format!("{}.yaml", name));
            if config_path.exists() {
                let content = tokio::fs::read_to_string(&config_path).await?;
                match loader(&content) {
                    Ok(config) => {
                        configs.insert(name.to_string(), config);
                        debug!("Loaded {} config", name);
                    }
                    Err(e) => {
                        warn!("Failed to parse {}.yaml: {}, using default", name, e);
                        if let Some(default_config) = default_configs.iter().find(|(n, _)| n == name).map(|(_, c)| c.clone()) {
                            configs.insert(name.to_string(), default_config);
                        }
                    }
                }
            } else {
                // Create default
                if let Some((_, default_config)) = default_configs.iter().find(|(n, _)| n == name) {
                    Self::save_system_config_yaml(config_dir, name, default_config).await?;
                    configs.insert(name.to_string(), default_config.clone());
                }
            }
        }

        info!("Loaded {} system configs", configs.len());
        Ok(configs)
    }

    /// Save system configuration as YAML
    async fn save_system_config_yaml(config_dir: &Path, name: &str, config: &SystemConfig) -> ConfigResult<()> {
        let config_path = config_dir.join(format!("{}.yaml", name));
        let content = match config {
            SystemConfig::Agent(c) => serde_yaml::to_string(c)?,
            SystemConfig::Logging(c) => serde_yaml::to_string(c)?,
            SystemConfig::Monitoring(c) => serde_yaml::to_string(c)?,
            SystemConfig::Compressor(c) => serde_yaml::to_string(c)?,
            SystemConfig::Storage(c) => serde_yaml::to_string(c)?,
            SystemConfig::Security(c) => serde_yaml::to_string(c)?,
        };
        tokio::fs::write(&config_path, content).await?;
        info!("Saved system config to: {}", config_path.display());
        Ok(())
    }

    /// Save system configuration (legacy, for backward compatibility)
    async fn save_system_config<T>(config_dir: &Path, name: &str, config: &T) -> ConfigResult<()>
    where
        T: serde::Serialize,
    {
        let config_path = config_dir.join(format!("{}.json", name));
        let content = serde_json::to_string_pretty(config)?;
        tokio::fs::write(&config_path, content).await?;
        info!("Saved system config to: {}", config_path.display());
        Ok(())
    }

    /// Setup file watcher for hot reload
    fn setup_watcher(
        config_dir: PathBuf,
        main_config: Arc<RwLock<KnightConfig>>,
        system_configs: Arc<RwLock<HashMap<String, SystemConfig>>>,
        change_tx: broadcast::Sender<ConfigChangeEvent>,
    ) -> ConfigResult<RecommendedWatcher> {
        use notify::{Event, EventKind};

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                        for path in &event.paths {
                            let filename = path.file_name()
                                .and_then(|n: &std::ffi::OsStr| n.to_str())
                                .unwrap_or("");

                            // Watch knight.json (main config)
                            if filename == "knight.json" {
                                info!("Detected change in knight.json");
                                if let Ok(content) = std::fs::read_to_string(path) {
                                    if let Ok(config) = serde_json::from_str::<KnightConfig>(&content) {
                                        *main_config.write() = config.clone();
                                        let _ = change_tx.send(ConfigChangeEvent::MainConfigChanged(config));
                                    }
                                }
                            }
                            // Watch YAML system configs
                            else if filename.ends_with(".yaml") {
                                info!("Detected change in {}", filename);
                                let config_name = filename.trim_end_matches(".yaml");
                                if let Ok(content) = std::fs::read_to_string(path) {
                                    let sys_config = match config_name {
                                        "agent" => serde_yaml::from_str::<AgentConfig>(&content)
                                            .map(SystemConfig::Agent).ok(),
                                        "logging" => serde_yaml::from_str::<LoggingConfig>(&content)
                                            .map(SystemConfig::Logging).ok(),
                                        "monitoring" => serde_yaml::from_str::<MonitoringConfig>(&content)
                                            .map(SystemConfig::Monitoring).ok(),
                                        "compressor" => serde_yaml::from_str::<CompressorConfig>(&content)
                                            .map(SystemConfig::Compressor).ok(),
                                        "storage" => serde_yaml::from_str::<StorageConfig>(&content)
                                            .map(SystemConfig::Storage).ok(),
                                        "security" => serde_yaml::from_str::<SecurityConfig>(&content)
                                            .map(SystemConfig::Security).ok(),
                                        _ => None,
                                    };

                                    if let Some(sc) = sys_config {
                                        system_configs.write().insert(config_name.to_string(), sc.clone());
                                        let _ = change_tx.send(ConfigChangeEvent::SystemConfigChanged {
                                            name: config_name.to_string(),
                                            config: sc,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Config watcher error: {}", e);
                }
            }
        })?;

        watcher.watch(&config_dir, RecursiveMode::Recursive)?;
        info!("Started watching config directory: {}", config_dir.display());

        Ok(watcher)
    }

    /// Get main configuration
    pub fn get_main_config(&self) -> KnightConfig {
        self.main_config.read().clone()
    }

    /// Get LLM configuration
    pub fn get_llm_config(&self) -> Option<LlmConfig> {
        self.main_config.read().llm.clone()
    }

    /// Get storage configuration
    pub fn get_storage_config(&self) -> StorageConfig {
        match self.get_system_config("storage") {
            Some(SystemConfig::Storage(c)) => c,
            _ => StorageConfig::default(),
        }
    }

    /// Get security configuration
    pub fn get_security_config(&self) -> SecurityConfig {
        match self.get_system_config("security") {
            Some(SystemConfig::Security(c)) => c,
            _ => SecurityConfig::default(),
        }
    }

    /// Get agent configuration
    pub fn get_agent_config(&self) -> AgentConfig {
        match self.get_system_config("agent") {
            Some(SystemConfig::Agent(c)) => c,
            _ => AgentConfig::default(),
        }
    }

    /// Get system configuration
    pub fn get_system_config(&self, name: &str) -> Option<SystemConfig> {
        self.system_configs.read().get(name).cloned()
    }

    /// Get logging configuration
    pub fn get_logging_config(&self) -> LoggingConfig {
        match self.get_system_config("logging") {
            Some(SystemConfig::Logging(c)) => c,
            _ => LoggingConfig::default(),
        }
    }

    /// Get monitoring configuration
    pub fn get_monitoring_config(&self) -> MonitoringConfig {
        match self.get_system_config("monitoring") {
            Some(SystemConfig::Monitoring(c)) => c,
            _ => MonitoringConfig::default(),
        }
    }

    /// Get compressor configuration
    pub fn get_compressor_config(&self) -> CompressorConfig {
        match self.get_system_config("compressor") {
            Some(SystemConfig::Compressor(c)) => c,
            _ => CompressorConfig::default(),
        }
    }

    /// Subscribe to config change events
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_tx.subscribe()
    }

    /// Reload main configuration
    pub async fn reload_main_config(&self) -> ConfigResult<()> {
        let config = Self::load_main_config(&self.config_dir).await?;
        *self.main_config.write() = config.clone();
        let _ = self.change_tx.send(ConfigChangeEvent::MainConfigChanged(config));
        Ok(())
    }

    /// Get config directory path
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::new(temp_dir.path().to_path_buf()).await.unwrap();

        // Check main config
        let main_config = loader.get_main_config();
        assert!(main_config.llm.is_none());
        assert!(main_config.storage.is_none());

        // Check system configs
        let logging = loader.get_logging_config();
        assert_eq!(logging.level, "info");

        let monitoring = loader.get_monitoring_config();
        assert!(!monitoring.enabled);

        let compressor = loader.get_compressor_config();
        assert!(compressor.enabled);
        assert_eq!(compressor.threshold_tokens, 30000);
    }

    #[tokio::test]
    async fn test_load_llm_config() {
        let temp_dir = TempDir::new().unwrap();

        // Create knight.json with LLM config
        let config_path = temp_dir.path().join("knight.json");
        let config_json = r#"{
            "llm": {
                "defaultProvider": "anthropic",
                "providers": {
                    "anthropic": {
                        "type": "anthropic",
                        "apiKey": "${ANTHROPIC_API_KEY}",
                        "baseUrl": "https://api.anthropic.com",
                        "timeoutSecs": 120,
                        "models": [{
                            "id": "claude-sonnet-4-6",
                            "contextLength": 200000,
                            "pricing": {"input": 3.0, "output": 15.0},
                            "capabilities": ["chat", "tools"]
                        }],
                        "defaultModel": "claude-sonnet-4-6"
                    }
                }
            }
        }"#;
        tokio::fs::write(&config_path, config_json).await.unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf()).await.unwrap();
        let llm_config = loader.get_llm_config();
        assert!(llm_config.is_some());

        let llm = llm_config.unwrap();
        assert_eq!(llm.default_provider, Some("anthropic".to_string()));
        assert!(llm.providers.contains_key("anthropic"));
    }
}
