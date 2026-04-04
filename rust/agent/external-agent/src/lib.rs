//! External Agent
//!
//! Design Reference: docs/03-module-design/agent/external-agent.md
//!
//! This module provides external agent integration capabilities:
//! - External agent discovery and availability checking
//! - Process lifecycle management
//! - Input/output handling
//! - Security validation

pub mod types;
pub mod manager;

pub use types::{
    ExternalAgentError, ExternalAgentResult, ProcessState, InputMode,
    ExternalAgentConfig, DiscoveredAgent, ExternalAgentStatus, AgentDefinition,
};

pub use manager::ExternalAgentManager;
