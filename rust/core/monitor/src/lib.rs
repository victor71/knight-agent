//! Monitor
//!
//! Design Reference: docs/03-module-design/core/monitor.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Monitor not initialized")]
    NotInitialized,
    #[error("Metric collection failed: {0}")]
    CollectionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_sessions: usize,
}

pub trait Monitor: Send + Sync {
    fn new() -> Result<Self, MonitorError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn collect_metrics(&self) -> Result<Metrics, MonitorError>;
    async fn start_monitoring(&self) -> Result<(), MonitorError>;
    async fn stop_monitoring(&self) -> Result<(), MonitorError>;
}

pub struct MonitorImpl;

impl Monitor for MonitorImpl {
    fn new() -> Result<Self, MonitorError> {
        Ok(MonitorImpl)
    }

    fn name(&self) -> &str {
        "monitor"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn collect_metrics(&self) -> Result<Metrics, MonitorError> {
        Ok(Metrics {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            active_sessions: 0,
        })
    }

    async fn start_monitoring(&self) -> Result<(), MonitorError> {
        Ok(())
    }

    async fn stop_monitoring(&self) -> Result<(), MonitorError> {
        Ok(())
    }
}
