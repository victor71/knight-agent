//! Workflows Directory
//!
//! Design Reference: docs/03-module-design/agent/workflows-directory.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkflowDirectoryError {
    #[error("Workflow directory not initialized")]
    NotInitialized,
    #[error("Workflow not found: {0}")]
    NotFound(String),
    #[error("Workflow registration failed: {0}")]
    RegistrationFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: String,
    pub action: String,
    pub parameters: serde_json::Value,
}

#[async_trait]
pub trait WorkflowDirectory: Send + Sync {
    fn new() -> Result<Self, WorkflowDirectoryError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn register_workflow(&self, workflow: Workflow) -> Result<(), WorkflowDirectoryError>;
    async fn get_workflow(&self, name: &str) -> Result<Workflow, WorkflowDirectoryError>;
    async fn list_workflows(&self) -> Result<Vec<Workflow>, WorkflowDirectoryError>;
}

pub struct WorkflowDirectoryImpl;

impl WorkflowDirectory for WorkflowDirectoryImpl {
    fn new() -> Result<Self, WorkflowDirectoryError> {
        Ok(WorkflowDirectoryImpl)
    }

    fn name(&self) -> &str {
        "workflows-directory"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn register_workflow(&self, _workflow: Workflow) -> Result<(), WorkflowDirectoryError> {
        Ok(())
    }

    async fn get_workflow(&self, name: &str) -> Result<Workflow, WorkflowDirectoryError> {
        Err(WorkflowDirectoryError::NotFound(name.to_string()))
    }

    async fn list_workflows(&self) -> Result<Vec<Workflow>, WorkflowDirectoryError> {
        Ok(vec![])
    }
}
