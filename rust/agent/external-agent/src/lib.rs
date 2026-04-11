//! External Agent
//!
//! Design Reference: docs/03-module-design/agent/external-agent.md
//!
//! This module provides external agent integration capabilities:
//! - External agent discovery and availability checking
//! - Process lifecycle management
//! - Input/output handling
//! - Security validation

pub mod manager;
pub mod types;

pub use types::{
    AgentDefinition, DiscoveredAgent, ExternalAgentConfig, ExternalAgentError, ExternalAgentResult,
    ExternalAgentStatus, InputMode, ProcessState,
};

pub use manager::ExternalAgentManager;
