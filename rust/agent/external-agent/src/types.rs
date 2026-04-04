//! External Agent Types
//!
//! Core data types for external agent management.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// External agent errors
#[derive(Error, Debug)]
pub enum ExternalAgentError {
    #[error("External agent not initialized")]
    NotInitialized,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Communication failed: {0}")]
    CommunicationFailed(String),
    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),
    #[error("Process not found: {0}")]
    ProcessNotFound(String),
    #[error("Stdin not available")]
    StdinNotAvailable,
    #[error("Process timeout")]
    ProcessTimeout,
    #[error("Process crashed: {0}")]
    ProcessCrashed(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Agent not installed: {0}")]
    AgentNotInstalled(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
}

/// Result type for external agent operations
pub type ExternalAgentResult<T> = Result<T, ExternalAgentError>;

/// External agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    Starting,
    Running,
    WaitingInput,
    Completed,
    Error,
    Killed,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::Starting
    }
}

/// Input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    Interactive,
    Batch,
    Pipe,
}

impl Default for InputMode {
    fn default() -> Self {
        Self::Pipe
    }
}

/// External agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAgentConfig {
    pub agent_type: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_stream_output")]
    pub stream_output: bool,
    #[serde(default)]
    pub input_mode: InputMode,
}

fn default_timeout() -> u64 {
    600
}

fn default_stream_output() -> bool {
    true
}

impl Default for ExternalAgentConfig {
    fn default() -> Self {
        Self {
            agent_type: "unknown".to_string(),
            command: String::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            working_dir: None,
            timeout: 600,
            stream_output: true,
            input_mode: InputMode::Pipe,
        }
    }
}

/// Discovered agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredAgent {
    pub agent_type: String,
    pub name: String,
    pub available: bool,
    pub installed: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub install_url: Option<String>,
}

impl DiscoveredAgent {
    pub fn new(agent_type: &str, name: &str) -> Self {
        Self {
            agent_type: agent_type.to_string(),
            name: name.to_string(),
            available: false,
            installed: false,
            version: None,
            path: None,
            reason: None,
            install_url: None,
        }
    }

    pub fn with_installed(mut self, installed: bool, path: Option<String>, version: Option<String>) -> Self {
        self.installed = installed;
        self.path = path;
        self.version = version;
        self.available = installed;
        self
    }

    pub fn with_unavailable(mut self, reason: &str, install_url: Option<String>) -> Self {
        self.available = false;
        self.reason = Some(reason.to_string());
        self.install_url = install_url;
        self
    }
}

/// External agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAgentStatus {
    pub agent_id: String,
    #[serde(default)]
    pub process_id: Option<String>,
    pub state: ProcessState,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub last_output_at: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub output_lines: u64,
    #[serde(default)]
    pub memory_mb: f64,
    #[serde(default)]
    pub cpu_percent: f64,
}

impl ExternalAgentStatus {
    pub fn new(agent_id: String, state: ProcessState) -> Self {
        Self {
            agent_id,
            process_id: None,
            state,
            started_at: None,
            last_output_at: None,
            exit_code: None,
            output_lines: 0,
            memory_mb: 0.0,
            cpu_percent: 0.0,
        }
    }

    pub fn with_process_id(mut self, process_id: String) -> Self {
        self.process_id = Some(process_id);
        self
    }

    pub fn with_started_at(mut self) -> Self {
        self.started_at = Some(chrono::Utc::now().to_rfc3339());
        self
    }
}

/// Agent definition for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub agent_type: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub version_flags: Vec<String>,
    pub install_url: String,
    pub install_instructions: String,
}

impl AgentDefinition {
    pub fn new(
        agent_type: &str,
        name: &str,
        command: &str,
        install_url: &str,
        install_instructions: &str,
    ) -> Self {
        Self {
            agent_type: agent_type.to_string(),
            name: name.to_string(),
            command: command.to_string(),
            version_flags: vec!["--version".to_string()],
            install_url: install_url.to_string(),
            install_instructions: install_instructions.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_agent_new() {
        let agent = DiscoveredAgent::new("claude-code", "Claude Code");
        assert_eq!(agent.agent_type, "claude-code");
        assert_eq!(agent.name, "Claude Code");
        assert!(!agent.available);
        assert!(!agent.installed);
    }

    #[test]
    fn test_discovered_agent_with_installed() {
        let agent = DiscoveredAgent::new("claude-code", "Claude Code")
            .with_installed(true, Some("/usr/bin/claude".to_string()), Some("1.2.3".to_string()));
        assert!(agent.installed);
        assert!(agent.available);
        assert_eq!(agent.path, Some("/usr/bin/claude".to_string()));
        assert_eq!(agent.version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_external_agent_config_default() {
        let config = ExternalAgentConfig::default();
        assert_eq!(config.timeout, 600);
        assert!(config.stream_output);
        assert_eq!(config.input_mode, InputMode::Pipe);
    }

    #[test]
    fn test_external_agent_status() {
        let status = ExternalAgentStatus::new("agent-1".to_string(), ProcessState::Running);
        assert_eq!(status.state, ProcessState::Running);
        assert!(status.process_id.is_none());
    }
}
