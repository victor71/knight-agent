//! TUI Event System
//!
//! Defines the application events and the event handler for the TUI.

use crate::state::{
    ContextCompressionStatus, OutputLine, SessionListItem, SessionTokenUsage, TaskInfo,
};
use crossterm::event::KeyEvent;
use std::time::Duration;

/// Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Input events
    Input(KeyEvent),
    Paste(String),

    // System events
    Tick,
    Resize { columns: u16, rows: u16 },

    // Status updates (from Monitor, ConfigLoader, etc.)
    SystemStatusUpdate(SystemStatusSnapshot),
    AgentUpdate(Vec<AgentInfo>),
    SessionUpdate(SessionInfo),
    ConfigChange(ConfigChangeEvent),

    // Output events
    OutputLine(OutputLine),
    StreamChunk(String), // For LLM streaming
    ClearOutput,

    // Session events
    SessionListUpdate(Vec<SessionListItem>),
    SessionSwitch(String),

    // Task events
    TaskListUpdate(Vec<TaskInfo>),
    TaskStart(String), // task_id
    TaskComplete(String),
    TaskDurationUpdate(Duration),

    // Session metrics events
    TokenUsageUpdate(SessionTokenUsage),
    ContextCompressionUpdate(ContextCompressionStatus),

    // Processing state events
    StartProcessing,
    StopProcessing,

    // Agent routing event (triggered from sync context)
    RouteToAgent(String),

    // Exit event (triggered by /quit command)
    Exit,
}

/// System status snapshot for rendering
#[derive(Debug, Clone)]
pub struct SystemStatusSnapshot {
    pub system_status: SystemHealth,
    pub stage: String,
    pub module_count: usize,
    pub initialized_count: usize,
    pub uptime: Duration,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHealth {
    Healthy,
    Degraded,
    Error,
}

/// Agent info for display
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub current_task: Option<String>,
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Busy,
    Error,
    Offline,
}

/// Session info for display
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub message_count: usize,
}

/// Config change event
#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    MainConfigChanged,
    SystemConfigChanged { name: String },
}

use tokio::sync::mpsc;

/// Event handler - manages the event channel receiver
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    /// Create a new event handler with its own channel
    pub fn new() -> (mpsc::UnboundedSender<AppEvent>, Self) {
        let (tx, rx) = mpsc::unbounded_channel();
        (tx, Self { rx })
    }

    /// Get the next event
    pub async fn next(&mut self) -> AppEvent {
        self.rx
            .recv()
            .await
            .expect("Event channel should not close")
    }

    /// Try to get the next event without blocking
    pub fn try_next(&mut self) -> Option<AppEvent> {
        self.rx.try_recv().ok()
    }
}
