//! Sandbox
//!
//! Design Reference: docs/03-module-design/security/sandbox.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Sandbox not initialized")]
    NotInitialized,
    #[error("Sandbox creation failed: {0}")]
    CreationFailed(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub max_memory_mb: u64,
    pub max_cpu_percent: u64,
    pub timeout_secs: u64,
    pub network_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub success: bool,
    pub output: String,
    pub exit_code: i32,
}

pub trait Sandbox: Send + Sync {
    fn new(config: SandboxConfig) -> Result<Self, SandboxError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn execute(&self, code: &str) -> Result<SandboxResult, SandboxError>;
    async fn destroy(&self) -> Result<(), SandboxError>;
}

pub struct SandboxImpl {
    config: SandboxConfig,
}

impl Sandbox for SandboxImpl {
    fn new(config: SandboxConfig) -> Result<Self, SandboxError> {
        Ok(SandboxImpl { config })
    }

    fn name(&self) -> &str {
        "sandbox"
    }

    fn is_initialized(&self) -> bool {
        true
    }

    async fn execute(&self, _code: &str) -> Result<SandboxResult, SandboxError> {
        Ok(SandboxResult {
            success: true,
            output: String::new(),
            exit_code: 0,
        })
    }

    async fn destroy(&self) -> Result<(), SandboxError> {
        Ok(())
    }
}
