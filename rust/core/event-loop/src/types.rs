//! Event Loop Types
//!
//! Core data types for the event-driven architecture.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event priority (lower number = higher priority)
pub const DEFAULT_PRIORITY: u32 = 100;

/// Event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub id: String,
    /// Event type (e.g., "file_change", "timer_triggered", "git_commit")
    pub event_type: String,
    /// Event source identifier
    pub source: String,
    /// Event timestamp (ISO 8601)
    pub timestamp: String,
    /// Event payload data
    pub data: serde_json::Value,
    /// Event priority (default: 100)
    #[serde(default = "default_priority")]
    pub priority: u32,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_priority() -> u32 {
    DEFAULT_PRIORITY
}

impl Event {
    /// Create a new event
    pub fn new(id: impl Into<String>, event_type: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            event_type: event_type.into(),
            source: source.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({}),
            priority: DEFAULT_PRIORITY,
            metadata: HashMap::new(),
        }
    }

    /// Create an event with data
    pub fn with_data(
        id: impl Into<String>,
        event_type: impl Into<String>,
        source: impl Into<String>,
        data: impl Into<serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            event_type: event_type.into(),
            source: source.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: data.into(),
            priority: DEFAULT_PRIORITY,
            metadata: HashMap::new(),
        }
    }
}

/// Event source type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventSourceType {
    FileWatcher,
    GitWatcher,
    Custom,
    Timer,
}

impl Default for EventSourceType {
    fn default() -> Self {
        Self::Custom
    }
}

/// File watcher configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileWatcherConfig {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub debounce_ms: u64,
}

/// Git watcher configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitWatcherConfig {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub poll_interval_secs: u64,
}

/// Timer configuration (for receiving timer_triggered events)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimerConfig {
    #[serde(default = "default_timer_source")]
    pub source: String,
}

fn default_timer_source() -> String {
    "timer_system".to_string()
}

/// Custom event source configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomSourceConfig {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub poll_interval_secs: u64,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Event source definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSource {
    /// Unique source identifier
    pub id: String,
    /// Source name
    pub name: String,
    /// Source type
    #[serde(rename = "type")]
    pub source_type: EventSourceType,
    /// Whether the source is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// File watcher config (if type is file_watcher)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_watcher: Option<FileWatcherConfig>,
    /// Git watcher config (if type is git_watcher)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_watcher: Option<GitWatcherConfig>,
    /// Timer config (if type is timer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timer: Option<TimerConfig>,
    /// Custom source config (if type is custom)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<CustomSourceConfig>,
}

fn default_enabled() -> bool {
    true
}

impl EventSource {
    /// Create a new event source
    pub fn new(id: impl Into<String>, name: impl Into<String>, source_type: EventSourceType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            source_type,
            enabled: true,
            file_watcher: None,
            git_watcher: None,
            timer: None,
            custom: None,
        }
    }
}

/// Event source information (without sensitive config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSourceInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: EventSourceType,
    pub enabled: bool,
}

impl From<&EventSource> for EventSourceInfo {
    fn from(source: &EventSource) -> Self {
        Self {
            id: source.id.clone(),
            name: source.name.clone(),
            source_type: source.source_type.clone(),
            enabled: source.enabled,
        }
    }
}

/// Event handler type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandlerType {
    Skill,
    Hook,
    Webhook,
    Callback,
}

/// Skill handler configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillHandler {
    #[serde(default)]
    pub skill_id: String,
    #[serde(default)]
    pub args: HashMap<String, serde_json::Value>,
}

/// Hook handler configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookHandler {
    #[serde(default)]
    pub hook_id: String,
}

/// Webhook handler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookHandler {
    pub url: String,
    #[serde(default = "default_webhook_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_webhook_method() -> String {
    "POST".to_string()
}

/// Event handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandler {
    #[serde(rename = "type")]
    pub handler_type: HandlerType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<SkillHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<HookHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook: Option<WebhookHandler>,
}

/// Event filter conditions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
    #[serde(default)]
    pub conditions: HashMap<String, serde_json::Value>,
}

/// Listener error handling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerErrorHandling {
    #[serde(default = "default_true")]
    pub retry: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_true")]
    pub continue_on_error: bool,
    #[serde(default = "default_true")]
    pub log_errors: bool,
}

fn default_true() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

impl Default for ListenerErrorHandling {
    fn default() -> Self {
        Self {
            retry: true,
            max_retries: 3,
            continue_on_error: true,
            log_errors: true,
        }
    }
}

/// Event listener
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventListener {
    /// Unique listener identifier
    pub id: String,
    /// Listener name
    pub name: String,
    /// Whether the listener is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Event filter
    #[serde(default)]
    pub filter: EventFilter,
    /// Event handler
    pub handler: EventHandler,
    /// Error handling configuration
    #[serde(default)]
    pub error_handling: ListenerErrorHandling,
}

impl EventListener {
    /// Create a new listener
    pub fn new(id: impl Into<String>, name: impl Into<String>, handler: EventHandler) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            enabled: true,
            filter: EventFilter::default(),
            handler,
            error_handling: ListenerErrorHandling::default(),
        }
    }
}

/// Event listener information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventListenerInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub filter: EventFilter,
}

impl From<&EventListener> for EventListenerInfo {
    fn from(listener: &EventListener) -> Self {
        Self {
            id: listener.id.clone(),
            name: listener.name.clone(),
            enabled: listener.enabled,
            filter: listener.filter.clone(),
        }
    }
}

/// Event queue overflow policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPolicy {
    Block,
    DropOldest,
    DropNewest,
}

impl Default for OverflowPolicy {
    fn default() -> Self {
        Self::Block
    }
}

/// Event loop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLoopConfig {
    /// Queue size
    #[serde(default = "default_queue_size")]
    pub queue_size: usize,
    /// Overflow policy
    #[serde(default)]
    pub overflow_policy: OverflowPolicy,
    /// Number of worker tasks
    #[serde(default = "default_workers")]
    pub workers: usize,
    /// Batch processing size
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Metrics enabled
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
}

fn default_queue_size() -> usize {
    10000
}

fn default_workers() -> usize {
    4
}

fn default_batch_size() -> usize {
    10
}

impl Default for EventLoopConfig {
    fn default() -> Self {
        Self {
            queue_size: 10000,
            overflow_policy: OverflowPolicy::Block,
            workers: 4,
            batch_size: 10,
            metrics_enabled: true,
        }
    }
}

/// Event loop status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLoopStatus {
    /// Whether the loop is running
    pub running: bool,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Total events processed
    pub events_processed: u64,
    /// Events per second
    pub events_per_second: f64,
    /// Number of active sources
    pub active_sources: usize,
    /// Number of active listeners
    pub active_listeners: usize,
}

/// Event statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    /// Total events received
    pub total_events: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Events by source
    pub events_by_source: HashMap<String, u64>,
    /// Average processing time in ms
    pub processing_time_avg_ms: f64,
    /// Error count
    pub error_count: u64,
}

/// Queue information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueInfo {
    /// Current queue size
    pub size: usize,
    /// Queue capacity
    pub capacity: usize,
    /// Utilization percentage
    pub utilization_percent: f64,
    /// Age of oldest event in ms
    pub oldest_event_age_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("1", "test_event", "test_source");
        assert_eq!(event.id, "1");
        assert_eq!(event.event_type, "test_event");
        assert_eq!(event.source, "test_source");
        assert_eq!(event.priority, 100);
    }

    #[test]
    fn test_event_source_creation() {
        let source = EventSource::new("src1", "Test Source", EventSourceType::FileWatcher);
        assert_eq!(source.id, "src1");
        assert!(source.enabled);
    }

    #[test]
    fn test_event_listener_creation() {
        let handler = EventHandler {
            handler_type: HandlerType::Skill,
            skill: Some(SkillHandler {
                skill_id: "test_skill".to_string(),
                args: HashMap::new(),
            }),
            hook: None,
            webhook: None,
        };
        let listener = EventListener::new("lst1", "Test Listener", handler);
        assert_eq!(listener.id, "lst1");
        assert!(listener.enabled);
    }

    #[test]
    fn test_event_loop_config_default() {
        let config = EventLoopConfig::default();
        assert_eq!(config.queue_size, 10000);
        assert_eq!(config.workers, 4);
    }
}
