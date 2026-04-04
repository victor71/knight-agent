//! Orchestrator
//!
//! Design Reference: docs/03-module-design/core/orchestrator.md
//!
//! Manages agent pool, task allocation, and message routing.

pub mod types;
pub mod manager;

pub use types::{
    OrchestratorError, OrchestratorResult, AgentStatus, AgentInfo, AgentStatistics,
    TaskRequirements, ResourceUsage, Collaboration, CollaborationMode, AgentMessage,
    SendResult, TopicSubscription, TopicMessage, OrchestratorConfig, SchedulingStrategy,
    AgentFilter,
};

pub use manager::OrchestratorImpl;
