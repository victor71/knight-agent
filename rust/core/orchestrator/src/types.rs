//! Orchestrator Types
//!
//! Core data types for the orchestrator module.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Orchestrator errors
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Orchestrator not initialized")]
    NotInitialized,
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Agent not available: {0}")]
    AgentNotAvailable(String),
    #[error("Task allocation failed: {0}")]
    AllocationFailed(String),
    #[error("Registration failed: {0}")]
    RegistrationFailed(String),
    #[error("Collaboration not found: {0}")]
    CollaborationNotFound(String),
    #[error("Message delivery failed: {0}")]
    MessageDeliveryFailed(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
}

/// Result type for orchestrator operations
pub type OrchestratorResult<T> = Result<T, OrchestratorError>;

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Busy,
    Paused,
    Error,
    Stopped,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub definition_id: String,
    pub session_id: String,
    #[serde(default)]
    pub variant: Option<String>,
    pub status: AgentStatus,
    #[serde(default)]
    pub current_task: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub statistics: AgentStatistics,
    pub created_at: String,
    #[serde(default)]
    pub last_active_at: Option<String>,
}

impl AgentInfo {
    pub fn new(id: String, name: String, definition_id: String, session_id: String) -> Self {
        Self {
            id,
            name,
            definition_id,
            session_id,
            variant: None,
            status: AgentStatus::Idle,
            current_task: None,
            capabilities: Vec::new(),
            statistics: AgentStatistics::default(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_active_at: None,
        }
    }

    pub fn with_variant(mut self, variant: &str) -> Self {
        self.variant = Some(variant.to_string());
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }
}

/// Agent statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStatistics {
    #[serde(default)]
    pub tasks_completed: u64,
    #[serde(default)]
    pub tasks_failed: u64,
    #[serde(default)]
    pub total_execution_time_ms: u64,
    #[serde(default)]
    pub memory_mb: f64,
    #[serde(default)]
    pub cpu_percent: f64,
}

/// Task requirements for agent allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub agent_variant: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub min_memory: Option<u64>,
    #[serde(default)]
    pub max_duration: Option<u64>,
    #[serde(default = "default_create_if_missing")]
    pub create_if_missing: bool,
}

fn default_create_if_missing() -> bool {
    true
}

impl Default for TaskRequirements {
    fn default() -> Self {
        Self {
            agent_type: None,
            agent_variant: None,
            capabilities: Vec::new(),
            min_memory: None,
            max_duration: None,
            create_if_missing: true,
        }
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    #[serde(default)]
    pub total_agents: usize,
    #[serde(default)]
    pub active_agents: usize,
    #[serde(default)]
    pub pending_tasks: usize,
    #[serde(default)]
    pub running_tasks: usize,
    #[serde(default)]
    pub memory_mb: u64,
    #[serde(default)]
    pub cpu_percent: f64,
}

/// Collaboration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationMode {
    MasterWorker,
    Pipeline,
    Voting,
}

impl Default for CollaborationMode {
    fn default() -> Self {
        Self::MasterWorker
    }
}

/// Collaboration group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaboration {
    pub id: String,
    pub name: String,
    pub agents: Vec<String>,
    pub mode: CollaborationMode,
    #[serde(default)]
    pub master: Option<String>,
    #[serde(default)]
    pub pipeline: Vec<String>,
    pub created_at: String,
}

impl Collaboration {
    pub fn new(id: &str, name: &str, agents: Vec<String>, mode: CollaborationMode) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            agents,
            mode,
            master: None,
            pipeline: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_master(mut self, master: &str) -> Self {
        self.master = Some(master.to_string());
        self
    }

    pub fn with_pipeline(mut self, pipeline: Vec<String>) -> Self {
        self.pipeline = pipeline;
        self
    }
}

/// Message to send to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub content: serde_json::Value,
    #[serde(default)]
    pub message_type: String,
    #[serde(default)]
    pub timestamp: String,
}

impl AgentMessage {
    pub fn new(from: &str, to: &str, content: serde_json::Value) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            content,
            message_type: "direct".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Send result for message delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub agent_id: String,
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
}

impl SendResult {
    pub fn success(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            success: true,
            error: None,
        }
    }

    pub fn failure(agent_id: &str, error: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            success: false,
            error: Some(error.to_string()),
        }
    }
}

/// Topic subscription
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSubscription {
    pub agent_id: String,
    pub topic: String,
    pub subscribed_at: String,
}

impl TopicSubscription {
    pub fn new(agent_id: &str, topic: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            topic: topic.to_string(),
            subscribed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Message published to a topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMessage {
    pub topic: String,
    pub from: String,
    pub content: serde_json::Value,
    pub timestamp: String,
}

impl TopicMessage {
    pub fn new(topic: &str, from: &str, content: serde_json::Value) -> Self {
        Self {
            topic: topic.to_string(),
            from: from.to_string(),
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    #[serde(default = "default_max_agents_per_session")]
    pub max_agents_per_session: usize,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,
    #[serde(default = "default_scheduling_strategy")]
    pub scheduling_strategy: SchedulingStrategy,
}

fn default_max_agents() -> usize {
    50
}

fn default_max_agents_per_session() -> usize {
    10
}

fn default_max_concurrent_tasks() -> usize {
    100
}

fn default_scheduling_strategy() -> SchedulingStrategy {
    SchedulingStrategy::RoundRobin
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_agents: 50,
            max_agents_per_session: 10,
            max_concurrent_tasks: 100,
            scheduling_strategy: SchedulingStrategy::RoundRobin,
        }
    }
}

/// Scheduling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulingStrategy {
    RoundRobin,
    LeastBusy,
    Priority,
}

impl Default for SchedulingStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Agent filter for querying agents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentFilter {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub status: Option<AgentStatus>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub definition_id: Option<String>,
    #[serde(default)]
    pub variant: Option<String>,
}
