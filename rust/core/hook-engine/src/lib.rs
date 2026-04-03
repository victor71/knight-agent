//! Hook Engine
//!
//! Design Reference: docs/03-module-design/core/hook-engine.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HookEngineError {
    #[error("Hook engine not initialized")]
    NotInitialized,
    #[error("Hook execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Hook not found: {0}")]
    HookNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub phase: HookPhase,
    pub callback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookPhase {
    PreExecution,
    PostExecution,
    OnError,
    OnSuccess,
}

pub trait HookEngine: Send + Sync {
    fn new() -> Result<Self, HookEngineError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn register_hook(&self, hook: Hook) -> Result<(), HookEngineError>;
    async fn execute_hooks(&self, phase: HookPhase) -> Result<(), HookEngineError>;
}

pub struct HookEngineImpl;

impl HookEngine for HookEngineImpl {
    fn new() -> Result<Self, HookEngineError> {
        Ok(HookEngineImpl)
    }

    fn name(&self) -> &str {
        "hook-engine"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn register_hook(&self, _hook: Hook) -> Result<(), HookEngineError> {
        Ok(())
    }

    async fn execute_hooks(&self, _phase: HookPhase) -> Result<(), HookEngineError> {
        Ok(())
    }
}
