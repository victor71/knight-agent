//! TUI Application State
//!
//! Defines the main application state for the TUI.

use crate::event::{AgentInfo, SessionInfo, SystemStatusSnapshot};
use crate::state::*;
use chrono::{DateTime, Local};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::event::AppEvent;

/// Main application state
pub struct AppState {
    // UI state
    pub terminal_size: (u16, u16),
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub active_popup: Option<PopupType>,
    pub popup_selected_index: usize,

    // Output state
    pub output_lines: Vec<OutputLine>,
    pub output_scroll: usize,
    pub max_output_lines: usize,

    // System state cache (snapshots for rendering)
    pub system_status: SystemStatusSnapshot,
    pub agents: Vec<AgentInfo>,
    pub session_info: SessionInfo,
    pub project_info: ProjectInfo,

    // Session management
    pub sessions: Vec<SessionListItem>,
    pub selected_session_index: usize,

    // Task management
    pub tasks: Vec<TaskInfo>,
    pub current_task_start: Option<DateTime<Local>>,
    pub current_task_duration: Option<Duration>,

    // Session metrics
    pub session_token_usage: SessionTokenUsage,
    pub context_compression_status: ContextCompressionStatus,

    // Channels
    pub event_tx: mpsc::UnboundedSender<AppEvent>,

    // Time
    pub current_time: DateTime<Local>,
}

/// Input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal, // Command mode
    Insert, // Text editing
    Visual, // Selection (future)
}

/// Popup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupType {
    SessionList,
    TaskList,
}

impl AppState {
    /// Create a new application state
    pub fn new(event_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self {
            terminal_size: (80, 24),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            active_popup: None,
            popup_selected_index: 0,
            output_lines: Vec::new(),
            output_scroll: 0,
            max_output_lines: 1000,
            system_status: SystemStatusSnapshot::default(),
            agents: Vec::new(),
            session_info: SessionInfo::default(),
            project_info: ProjectInfo::default(),
            sessions: Vec::new(),
            selected_session_index: 0,
            tasks: Vec::new(),
            current_task_start: None,
            current_task_duration: None,
            session_token_usage: SessionTokenUsage::new(0, 200_000),
            context_compression_status: ContextCompressionStatus::new(0, 25 * 1024 * 1024),
            event_tx,
            current_time: Local::now(),
        }
    }

    /// Update state based on event
    pub fn update(&mut self, event: &AppEvent) {
        match event {
            AppEvent::Tick => {
                self.current_time = Local::now();
            }
            AppEvent::Resize { columns, rows } => {
                self.terminal_size = (*columns, *rows);
            }
            AppEvent::OutputLine(line) => {
                self.add_output_line(line.clone());
            }
            AppEvent::StreamChunk(chunk) => {
                // Append to last line if it exists and is a stream, otherwise create new
                if let Some(last) = self.output_lines.last_mut() {
                    if matches!(last.style, OutputStyle::AgentMessage) {
                        last.content.push_str(chunk);
                        return;
                    }
                }
                self.add_output_line(OutputLine {
                    content: chunk.clone(),
                    style: OutputStyle::AgentMessage,
                    timestamp: Local::now(),
                });
            }
            AppEvent::ClearOutput => {
                self.output_lines.clear();
                self.output_scroll = 0;
            }
            AppEvent::SessionListUpdate(sessions) => {
                self.sessions = sessions.clone();
            }
            AppEvent::TaskListUpdate(tasks) => {
                self.tasks = tasks.clone();
            }
            AppEvent::TaskDurationUpdate(duration) => {
                self.current_task_duration = Some(*duration);
            }
            AppEvent::TokenUsageUpdate(usage) => {
                self.session_token_usage = usage.clone();
            }
            AppEvent::ContextCompressionUpdate(status) => {
                self.context_compression_status = status.clone();
            }
            _ => {}
        }
    }

    /// Add an output line, respecting max_lines limit
    fn add_output_line(&mut self, line: OutputLine) {
        if self.output_lines.len() >= self.max_output_lines {
            self.output_lines.remove(0);
        }
        self.output_lines.push(line);
    }

    /// Get the formatted current task duration
    pub fn current_task_duration_formatted(&self) -> String {
        if let Some(duration) = self.current_task_duration {
            let secs = duration.as_secs();
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            let secs = secs % 60;
            format!("{:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            "--:--:--".to_string()
        }
    }

    /// Format bytes for display
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if bytes >= GB {
            format!("{:.1}GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1}MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1}KB", bytes as f64 / KB as f64)
        } else {
            format!("{}B", bytes)
        }
    }

    /// Switch to insert mode
    pub fn enter_insert_mode(&mut self) {
        self.input_mode = InputMode::Insert;
    }

    /// Switch to normal mode
    pub fn enter_normal_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Toggle popup
    pub fn toggle_popup(&mut self, popup: PopupType) {
        if self.active_popup == Some(popup) {
            self.active_popup = None;
        } else {
            self.active_popup = Some(popup);
            self.popup_selected_index = 0;
        }
    }

    /// Close popup
    pub fn close_popup(&mut self) {
        self.active_popup = None;
        self.popup_selected_index = 0;
    }
}

impl Default for SystemStatusSnapshot {
    fn default() -> Self {
        Self {
            system_status: crate::event::SystemHealth::Healthy,
            stage: "Ready".to_string(),
            module_count: 0,
            initialized_count: 0,
            uptime: Duration::ZERO,
            cpu_usage: 0.0,
            memory_usage: 0,
        }
    }
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default Session".to_string(),
            created_at: Local::now(),
            message_count: 0,
        }
    }
}
