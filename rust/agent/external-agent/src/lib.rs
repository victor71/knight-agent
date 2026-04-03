//! External Agent
//!
//! Design Reference: docs/03-module-design/agent/external-agent.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExternalAgentError {
    #[error("External agent not initialized")]
    NotInitialized,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Communication failed: {0}")]
    CommunicationFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAgentConfig {
    pub endpoint: String,
    pub auth_token: Option<String>,
}

pub trait ExternalAgent: Send + Sync {
    fn new(config: ExternalAgentConfig) -> Result<Self, ExternalAgentError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn connect(&self) -> Result<(), ExternalAgentError>;
    async fn disconnect(&self) -> Result<(), ExternalAgentError>;
    async fn send_message(&self, message: serde_json::Value) -> Result<serde_json::Value, ExternalAgentError>;
}

pub struct ExternalAgentImpl {
    config: ExternalAgentConfig,
    connected: bool,
}

impl ExternalAgent for ExternalAgentImpl {
    fn new(config: ExternalAgentConfig) -> Result<Self, ExternalAgentError> {
        Ok(ExternalAgentImpl {
            config,
            connected: false,
        })
    }

    fn name(&self) -> &str {
        "external-agent"
    }

    fn is_initialized(&self) -> bool {
        self.connected
    }

    async fn connect(&self) -> Result<(), ExternalAgentError> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), ExternalAgentError> {
        Ok(())
    }

    async fn send_message(&self, _message: serde_json::Value) -> Result<serde_json::Value, ExternalAgentError> {
        Ok(serde_json::json!({}))
    }
}
