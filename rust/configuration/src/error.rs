//! Configuration error types

use std::path::PathBuf;
use thiserror::Error;

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config directory not found: {0}")]
    ConfigDirNotFound(PathBuf),

    #[error("Config file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse JSON config file: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Failed to parse YAML config file: {0}")]
    YamlParseError(#[from] serde_yaml::Error),

    #[error("Invalid config value: {0}")]
    InvalidValue(String),

    #[error("Config validation failed: {0}")]
    ValidationFailed(String),

    #[error("Failed to watch config file for changes: {0}")]
    WatchError(#[from] notify::Error),
}

/// Configuration result type
pub type ConfigResult<T> = Result<T, ConfigError>;
