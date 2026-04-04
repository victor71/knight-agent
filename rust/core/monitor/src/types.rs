//! Monitor Types
//!
//! Core data types for the monitoring system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Monitor errors
#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Monitor not initialized")]
    NotInitialized,
    #[error("Metric collection failed: {0}")]
    CollectionFailed(String),
    #[error("Stats not found: {0}")]
    StatsNotFound(String),
    #[error("Invalid scope: {0}")]
    InvalidScope(String),
}

/// Result type for monitor operations
pub type MonitorResult<T> = Result<T, MonitorError>;

/// Stat scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum StatScope {
    #[default]
    All,
    Session,
    Agent,
}


/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub total: u64,
    pub by_model: HashMap<String, u64>,
    pub by_type: HashMap<String, u64>,
}

impl TokenUsage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, amount: u64, model: &str, token_type: &str) {
        self.total += amount;
        *self.by_model.entry(model.to_string()).or_insert(0) += amount;
        *self.by_type.entry(token_type.to_string()).or_insert(0) += amount;
    }
}

/// Session statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub active_count: usize,
    pub total_count: usize,
    pub archived_count: usize,
    pub total_messages: u64,
}

/// Agent statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStats {
    pub active_count: usize,
    pub total_created: usize,
    pub total_tasks_completed: u64,
}

/// System resource stats
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemResourceStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
}

/// System statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemStats {
    pub tokens: TokenUsage,
    pub sessions: SessionStats,
    pub agents: AgentStats,
    pub resources: SystemResourceStats,
    pub uptime_seconds: u64,
    pub last_updated: String,
}

impl SystemStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tokens(mut self, tokens: TokenUsage) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn with_sessions(mut self, sessions: SessionStats) -> Self {
        self.sessions = sessions;
        self
    }

    pub fn with_agents(mut self, agents: AgentStats) -> Self {
        self.agents = agents;
        self
    }
}

/// System status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub running: bool,
    pub initialized: bool,
    pub stats: SystemStats,
}

/// Status scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum StatusScope {
    #[default]
    All,
    Session,
    Agent,
}


/// Status update for watching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub timestamp: String,
    pub scope: StatusScope,
    pub status: SystemStatus,
}

impl StatusUpdate {
    pub fn new(scope: StatusScope, status: SystemStatus) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            scope,
            status,
        }
    }
}

/// Historical stats entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalStats {
    pub period: String,
    pub start_time: String,
    pub end_time: String,
    pub tokens_used: u64,
    pub sessions_created: usize,
    pub agents_created: usize,
    pub avg_cpu_usage: f64,
    pub avg_memory_usage: f64,
}

/// Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_sessions: usize,
    pub timestamp: String,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            active_sessions: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
