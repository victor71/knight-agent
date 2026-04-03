//! Logging System Types
//!
//! Core data structures for the logging system.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::LoggingError;

/// Log level enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    #[allow(clippy::should_implement_trait)]
    pub fn parse(s: &str) -> Result<Self, LoggingError> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "fatal" | "critical" => Ok(LogLevel::Fatal),
            _ => Err(LoggingError::InvalidLevel(s.to_string())),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::Fatal => write!(f, "fatal"),
        }
    }
}

/// Log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: std::time::SystemTime,
    pub level: LogLevel,
    pub module: String,
    pub session_id: Option<String>,
    pub message: String,
    pub context: HashMap<String, serde_json::Value>,
    pub error: Option<ErrorInfo>,
}

/// Error information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
}

/// Log filter for querying logs
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub level: Option<LogLevel>,
    pub module: Option<String>,
    pub session_id: Option<String>,
    pub since: Option<std::time::SystemTime>,
    pub until: Option<std::time::SystemTime>,
    pub message_pattern: Option<String>,
}

/// Log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStats {
    pub total_entries: usize,
    pub entries_by_level: HashMap<String, usize>,
    pub entries_by_module: HashMap<String, usize>,
    pub oldest_entry: Option<std::time::SystemTime>,
    pub newest_entry: Option<std::time::SystemTime>,
}

/// Log context for additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

impl LogContext {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Export format enumeration
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
    Text,
}
