//! Agent Runtime
//!
//! Design Reference: docs/03-module-design/agent/agent-runtime.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentRuntimeError {
    #[error("Agent runtime not initialized")]
    NotInitialized,
    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub variant: String,
    pub state: AgentState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Running,
    Paused,
    Stopped,
}

pub trait AgentRuntime: Send + Sync {
    fn new() -> Result<Self, AgentRuntimeError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn spawn_agent(&self, variant: &str) -> Result<Agent, AgentRuntimeError>;
    async fn stop_agent(&self, id: &str) -> Result<(), AgentRuntimeError>;
    async fn pause_agent(&self, id: &str) -> Result<(), AgentRuntimeError>;
    async fn resume_agent(&self, id: &str) -> Result<(), AgentRuntimeError>;
}

pub struct AgentRuntimeImpl;

impl AgentRuntime for AgentRuntimeImpl {
    fn new() -> Result<Self, AgentRuntimeError> {
        Ok(AgentRuntimeImpl)
    }

    fn name(&self) -> &str {
        "agent-runtime"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn spawn_agent(&self, variant: &str) -> Result<Agent, AgentRuntimeError> {
        Ok(Agent {
            id: format!("agent-{}", uuid::Uuid::new_v4()),
            variant: variant.to_string(),
            state: AgentState::Idle,
        })
    }

    async fn stop_agent(&self, _id: &str) -> Result<(), AgentRuntimeError> {
        Ok(())
    }

    async fn pause_agent(&self, _id: &str) -> Result<(), AgentRuntimeError> {
        Ok(())
    }

    async fn resume_agent(&self, _id: &str) -> Result<(), AgentRuntimeError> {
        Ok(())
    }
}
