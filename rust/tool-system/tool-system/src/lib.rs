//! Tool System
//!
//! Design Reference: docs/03-module-design/tool-system/tool-system.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolSystemError {
    #[error("Tool system not initialized")]
    NotInitialized,
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

pub trait ToolSystem: Send + Sync {
    fn new() -> Result<Self, ToolSystemError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn register_tool(&self, tool: Tool) -> Result<(), ToolSystemError>;
    async fn execute_tool(&self, name: &str, params: serde_json::Value) -> Result<serde_json::Value, ToolSystemError>;
    async fn list_tools(&self) -> Result<Vec<Tool>, ToolSystemError>;
}

pub struct ToolSystemImpl;

impl ToolSystem for ToolSystemImpl {
    fn new() -> Result<Self, ToolSystemError> {
        Ok(ToolSystemImpl)
    }

    fn name(&self) -> &str {
        "tool-system"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn register_tool(&self, _tool: Tool) -> Result<(), ToolSystemError> {
        Ok(())
    }

    async fn execute_tool(&self, name: &str, _params: serde_json::Value) -> Result<serde_json::Value, ToolSystemError> {
        Err(ToolSystemError::NotFound(name.to_string()))
    }

    async fn list_tools(&self) -> Result<Vec<Tool>, ToolSystemError> {
        Ok(vec![])
    }
}
