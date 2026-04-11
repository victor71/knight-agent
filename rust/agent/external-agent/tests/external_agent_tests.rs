//! External Agent Tests
//!
//! Unit tests for the external-agent module.

use external_agent::{
    AgentDefinition, DiscoveredAgent, ExternalAgentConfig, ExternalAgentError,
    ExternalAgentManager, ExternalAgentStatus, InputMode, ProcessState,
};

#[tokio::test]
async fn test_manager_new() {
    let manager = ExternalAgentManager::new();
    assert_eq!(manager.agent_count().await, 0);
}

#[tokio::test]
async fn test_discover_agents() {
    let manager = ExternalAgentManager::new();
    let discovered = manager.discover().await;
    // Should discover at least claude-code
    assert!(!discovered.is_empty());
}

#[tokio::test]
async fn test_check_availability() {
    let manager = ExternalAgentManager::new();
    let result = manager.check_availability("claude-code").await;
    assert_eq!(result.agent_type, "claude-code");
}

#[tokio::test]
async fn test_check_availability_unknown() {
    let manager = ExternalAgentManager::new();
    let result = manager.check_availability("unknown-agent").await;
    assert!(!result.available);
    assert!(result.reason.is_some());
}

#[tokio::test]
async fn test_get_install_instructions() {
    let manager = ExternalAgentManager::new();
    let instructions = manager.get_install_instructions("claude-code");
    assert!(instructions.is_some());
    let instructions = instructions.unwrap();
    assert!(instructions.contains("Claude Code"));
}

#[tokio::test]
async fn test_get_install_instructions_unknown() {
    let manager = ExternalAgentManager::new();
    let instructions = manager.get_install_instructions("unknown-agent");
    assert!(instructions.is_none());
}

#[tokio::test]
async fn test_list_agents_empty() {
    let manager = ExternalAgentManager::new();
    let agents = manager.list_agents().await;
    assert!(agents.is_empty());
}

#[tokio::test]
async fn test_has_agent_empty() {
    let manager = ExternalAgentManager::new();
    assert!(!manager.has_agent("test-agent").await);
}

#[tokio::test]
async fn test_validate_input_normal() {
    let manager = ExternalAgentManager::new();
    let result = manager.validate_input("List files in current directory");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_input_too_large() {
    let manager = ExternalAgentManager::new();
    let large_input = "x".repeat(1_000_001);
    let result = manager.validate_input(&large_input);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_input_dangerous() {
    let manager = ExternalAgentManager::new();
    let dangerous_inputs = vec!["rm -rf /", "rm -rf /*", "format c:", "mkfs", ":(){:|:&};:"];

    for input in dangerous_inputs {
        let result = manager.validate_input(input);
        assert!(result.is_err(), "Should reject dangerous input: {}", input);
    }
}

#[test]
fn test_discovered_agent_new() {
    let agent = DiscoveredAgent::new("test-agent", "Test Agent");
    assert_eq!(agent.agent_type, "test-agent");
    assert_eq!(agent.name, "Test Agent");
    assert!(!agent.available);
    assert!(!agent.installed);
}

#[test]
fn test_discovered_agent_with_installed() {
    let agent = DiscoveredAgent::new("claude-code", "Claude Code").with_installed(
        true,
        Some("/usr/bin/claude".to_string()),
        Some("1.2.3".to_string()),
    );
    assert!(agent.installed);
    assert!(agent.available);
    assert_eq!(agent.path, Some("/usr/bin/claude".to_string()));
    assert_eq!(agent.version, Some("1.2.3".to_string()));
}

#[test]
fn test_discovered_agent_with_unavailable() {
    let agent = DiscoveredAgent::new("unknown", "Unknown Agent")
        .with_unavailable("Not found", Some("https://example.com".to_string()));
    assert!(!agent.available);
    assert_eq!(agent.reason, Some("Not found".to_string()));
    assert_eq!(agent.install_url, Some("https://example.com".to_string()));
}

#[test]
fn test_external_agent_config_default() {
    let config = ExternalAgentConfig::default();
    assert_eq!(config.timeout, 600);
    assert!(config.stream_output);
    assert_eq!(config.input_mode, InputMode::Pipe);
}

#[test]
fn test_external_agent_config_with_values() {
    let mut env = std::collections::HashMap::new();
    env.insert("API_KEY".to_string(), "secret".to_string());

    let config = ExternalAgentConfig {
        agent_type: "claude-code".to_string(),
        command: "claude".to_string(),
        args: vec!["--print".to_string()],
        env,
        working_dir: Some("/tmp".to_string()),
        timeout: 300,
        stream_output: false,
        input_mode: InputMode::Interactive,
    };

    assert_eq!(config.agent_type, "claude-code");
    assert_eq!(config.command, "claude");
    assert_eq!(config.args.len(), 1);
    assert_eq!(config.timeout, 300);
    assert!(!config.stream_output);
    assert_eq!(config.input_mode, InputMode::Interactive);
}

#[test]
fn test_external_agent_status_new() {
    let status = ExternalAgentStatus::new("agent-1".to_string(), ProcessState::Running);
    assert_eq!(status.agent_id, "agent-1");
    assert_eq!(status.state, ProcessState::Running);
    assert!(status.process_id.is_none());
    assert!(status.exit_code.is_none());
}

#[test]
fn test_external_agent_status_with_process_id() {
    let status = ExternalAgentStatus::new("agent-1".to_string(), ProcessState::Starting)
        .with_process_id("proc-123".to_string())
        .with_started_at();
    assert_eq!(status.process_id, Some("proc-123".to_string()));
    assert!(status.started_at.is_some());
}

#[test]
fn test_process_state_default() {
    let state = ProcessState::default();
    assert_eq!(state, ProcessState::Starting);
}

#[test]
fn test_input_mode_default() {
    let mode = InputMode::default();
    assert_eq!(mode, InputMode::Pipe);
}

#[test]
fn test_agent_definition_new() {
    let def = AgentDefinition::new(
        "claude-code",
        "Claude Code",
        "claude",
        "https://example.com",
        "Install instructions",
    );
    assert_eq!(def.agent_type, "claude-code");
    assert_eq!(def.name, "Claude Code");
    assert_eq!(def.command, "claude");
    assert_eq!(def.install_url, "https://example.com");
}

#[test]
fn test_external_agent_error_display() {
    let error = ExternalAgentError::ProcessNotFound("test-agent".to_string());
    assert_eq!(error.to_string(), "Process not found: test-agent");

    let error = ExternalAgentError::ProcessTimeout;
    assert_eq!(error.to_string(), "Process timeout");

    let error = ExternalAgentError::AgentNotInstalled("claude-code".to_string());
    assert_eq!(error.to_string(), "Agent not installed: claude-code");
}
