//! Timer System
//!
//! Design Reference: docs/03-module-design/services/timer-system.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TimerError {
    #[error("Timer system not initialized")]
    NotInitialized,
    #[error("Timer creation failed: {0}")]
    CreationFailed(String),
    #[error("Timer not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timer {
    pub id: String,
    pub duration_secs: u64,
    pub callback: String,
    pub repeating: bool,
}

#[async_trait]
pub trait TimerSystem: Send + Sync {
    fn new() -> Result<Self, TimerError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn create_timer(&self, timer: Timer) -> Result<String, TimerError>;
    async fn cancel_timer(&self, id: &str) -> Result<(), TimerError>;
    async fn list_timers(&self) -> Result<Vec<Timer>, TimerError>;
}

pub struct TimerSystemImpl;

impl TimerSystem for TimerSystemImpl {
    fn new() -> Result<Self, TimerError> {
        Ok(TimerSystemImpl)
    }

    fn name(&self) -> &str {
        "timer-system"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn create_timer(&self, timer: Timer) -> Result<String, TimerError> {
        Ok(timer.id)
    }

    async fn cancel_timer(&self, _id: &str) -> Result<(), TimerError> {
        Ok(())
    }

    async fn list_timers(&self) -> Result<Vec<Timer>, TimerError> {
        Ok(vec![])
    }
}
