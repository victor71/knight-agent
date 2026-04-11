//! Timer System Types
//!
//! Core data types for the timer system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use thiserror::Error;

/// Timer system errors
#[derive(Error, Debug)]
pub enum TimerError {
    #[error("Timer system not initialized")]
    NotInitialized,
    #[error("Timer creation failed: {0}")]
    CreationFailed(String),
    #[error("Timer not found: {0}")]
    NotFound(String),
}

/// Result type for timer operations
pub type TimerResult<T> = Result<T, TimerError>;

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimerType {
    #[default]
    Oneshot,
    Interval,
    Cron,
}

/// Timer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimerStatus {
    #[default]
    Pending,
    Active,
    Paused,
    Completed,
    Cancelled,
}

/// Callback type for timers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TimerCallback {
    Callback {
        #[serde(default)]
        handler: String,
    },
    Hook {
        hook_id: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Skill {
        skill_id: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Webhook {
        url: String,
        #[serde(default = "default_webhook_method")]
        method: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        body: serde_json::Value,
    },
}

fn default_webhook_method() -> String {
    "POST".to_string()
}

/// Timer configuration for oneshot timers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneshotConfig {
    pub delay_ms: u64,
    #[serde(default)]
    pub execute_at: Option<String>,
}

/// Timer configuration for interval timers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalConfig {
    pub interval_ms: u64,
    #[serde(default = "default_max_executions")]
    pub max_executions: i32,
    #[serde(default)]
    pub execution_count: u32,
    #[serde(default)]
    pub next_execution: Option<String>,
}

fn default_max_executions() -> i32 {
    -1
}

/// Timer configuration for cron timers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    pub expression: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub next_execution: Option<String>,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// Timer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timer {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub timer_type: TimerType,
    #[serde(default)]
    pub status: TimerStatus,
    pub callback: TimerCallback,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub last_execution: Option<String>,
    #[serde(default)]
    pub last_result: Option<TimerExecutionResult>,
    #[serde(flatten)]
    pub config: TimerConfig,
}

/// Timer configuration (one of the three types)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TimerConfig {
    Oneshot(OneshotConfig),
    Interval(IntervalConfig),
    Cron(CronConfig),
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self::Oneshot(OneshotConfig {
            delay_ms: 0,
            execute_at: None,
        })
    }
}

/// Timer execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerExecutionResult {
    pub status: ExecutionStatus,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Failed,
    Timeout,
}

/// Timer information (for queries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerInfo {
    pub id: String,
    pub name: String,
    pub timer_type: TimerType,
    pub status: TimerStatus,
    pub callback_type: String,
    pub created_at: String,
    pub next_execution: Option<String>,
    pub last_execution: Option<String>,
    #[serde(default)]
    pub execution_count: u32,
}

impl From<&Timer> for TimerInfo {
    fn from(timer: &Timer) -> Self {
        let callback_type = match &timer.callback {
            TimerCallback::Callback { .. } => "callback",
            TimerCallback::Hook { .. } => "hook",
            TimerCallback::Skill { .. } => "skill",
            TimerCallback::Webhook { .. } => "webhook",
        };

        let execution_count = match &timer.config {
            TimerConfig::Interval(config) => config.execution_count,
            _ => 0,
        };

        Self {
            id: timer.id.clone(),
            name: timer.name.clone(),
            timer_type: timer.timer_type,
            status: timer.status,
            callback_type: callback_type.to_string(),
            created_at: timer.created_at.clone(),
            next_execution: timer.next_execution(),
            last_execution: timer.last_execution.clone(),
            execution_count,
        }
    }
}

impl Timer {
    /// Create a new oneshot timer
    pub fn oneshot(id: String, delay_ms: u64, callback: TimerCallback) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name: String::new(),
            timer_type: TimerType::Oneshot,
            status: TimerStatus::Pending,
            callback,
            metadata: HashMap::new(),
            persistent: false,
            created_at: now.clone(),
            updated_at: now,
            last_execution: None,
            last_result: None,
            config: TimerConfig::Oneshot(OneshotConfig {
                delay_ms,
                execute_at: None,
            }),
        }
    }

    /// Create a new interval timer
    pub fn interval(id: String, interval_ms: u64, callback: TimerCallback) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name: String::new(),
            timer_type: TimerType::Interval,
            status: TimerStatus::Pending,
            callback,
            metadata: HashMap::new(),
            persistent: false,
            created_at: now.clone(),
            updated_at: now,
            last_execution: None,
            last_result: None,
            config: TimerConfig::Interval(IntervalConfig {
                interval_ms,
                max_executions: -1,
                execution_count: 0,
                next_execution: None,
            }),
        }
    }

    /// Create a new cron timer
    pub fn cron(id: String, expression: String, callback: TimerCallback) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name: String::new(),
            timer_type: TimerType::Cron,
            status: TimerStatus::Pending,
            callback,
            metadata: HashMap::new(),
            persistent: false,
            created_at: now.clone(),
            updated_at: now,
            last_execution: None,
            last_result: None,
            config: TimerConfig::Cron(CronConfig {
                expression,
                timezone: "UTC".to_string(),
                next_execution: None,
            }),
        }
    }

    /// Get the next execution time
    pub fn next_execution(&self) -> Option<String> {
        match &self.config {
            TimerConfig::Oneshot(config) => {
                if self.status == TimerStatus::Active || self.status == TimerStatus::Pending {
                    let delay = config.delay_ms;
                    let created = chrono::DateTime::parse_from_rfc3339(&self.created_at).ok()?;
                    let next = created + chrono::Duration::milliseconds(delay as i64);
                    Some(next.to_rfc3339())
                } else {
                    None
                }
            }
            TimerConfig::Interval(config) => config.next_execution.clone(),
            TimerConfig::Cron(config) => config.next_execution.clone(),
        }
    }
}

/// Timer statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimerStats {
    pub total_timers: u64,
    pub active_timers: u64,
    pub paused_timers: u64,
    pub completed_timers: u64,
    pub by_type: TimerStatsByType,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    #[serde(default)]
    pub avg_execution_time_ms: f64,
}

/// Timer statistics by type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimerStatsByType {
    pub oneshot: u64,
    pub interval: u64,
    pub cron: u64,
}

/// Timer execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerExecution {
    pub id: String,
    pub timer_id: String,
    pub executed_at: String,
    pub scheduled_at: String,
    pub delay_ms: i64,
    pub result: TimerExecutionResult,
}

/// Internal timer entry for the scheduler
#[derive(Debug, Clone)]
pub struct ScheduledTimer {
    pub timer: Timer,
    pub scheduled_at: Instant,
    pub next_execution: Instant,
}

impl ScheduledTimer {
    /// Check if the timer is ready to execute
    pub fn is_ready(&self) -> bool {
        self.next_execution <= Instant::now()
    }

    /// Time remaining until next execution
    pub fn time_remaining(&self) -> std::time::Duration {
        let elapsed = self.scheduled_at.elapsed();
        let delay = std::time::Duration::from_millis(self.delay_ms());
        if elapsed >= delay {
            std::time::Duration::ZERO
        } else {
            delay - elapsed
        }
    }

    /// Get the delay in milliseconds
    pub fn delay_ms(&self) -> u64 {
        match &self.timer.config {
            TimerConfig::Oneshot(config) => config.delay_ms,
            TimerConfig::Interval(config) => config.interval_ms,
            TimerConfig::Cron(_) => 0,
        }
    }
}

/// Filter for querying timers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimerFilter {
    #[serde(default)]
    pub timer_type: Option<TimerType>,
    #[serde(default)]
    pub status: Option<TimerStatus>,
    #[serde(default)]
    pub name_pattern: Option<String>,
    #[serde(default)]
    pub created_after: Option<String>,
    #[serde(default)]
    pub created_before: Option<String>,
    #[serde(default)]
    pub persistent: Option<bool>,
}
