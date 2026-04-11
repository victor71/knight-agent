//! Orchestrator
//!
//! Design Reference: docs/03-module-design/core/orchestrator.md
//!
//! Manages agent pool, task allocation, and message routing.

pub mod manager;
pub mod types;

pub use types::{
    AgentFilter, AgentInfo, AgentMessage, AgentStatistics, AgentStatus, Collaboration,
    CollaborationMode, OrchestratorConfig, OrchestratorError, OrchestratorResult, ResourceUsage,
    SchedulingStrategy, SendResult, TaskRequirements, TopicMessage, TopicSubscription,
};

pub use manager::OrchestratorImpl;
