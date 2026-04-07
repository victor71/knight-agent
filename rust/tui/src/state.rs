//! TUI State Types
//!
//! Defines state snapshot types for the TUI.

use chrono::{DateTime, Local};

/// Session token usage info
#[derive(Debug, Clone)]
pub struct SessionTokenUsage {
    pub current: u64,
    pub limit: u64,
    pub percentage: f32,
}

impl SessionTokenUsage {
    pub fn new(current: u64, limit: u64) -> Self {
        let percentage = if limit > 0 {
            (current as f32 / limit as f32) * 100.0
        } else {
            0.0
        };
        Self {
            current,
            limit,
            percentage,
        }
    }
}

/// Context compression status
#[derive(Debug, Clone)]
pub struct ContextCompressionStatus {
    pub current_size: u64,      // bytes
    pub threshold: u64,         // bytes (compression trigger)
    pub percentage: f32,        // current / threshold
    pub until_compression: f32, // (threshold - current) / threshold
}

impl ContextCompressionStatus {
    pub fn new(current_size: u64, threshold: u64) -> Self {
        let percentage = if threshold > 0 {
            (current_size as f32 / threshold as f32) * 100.0
        } else {
            0.0
        };
        let until_compression = 100.0 - percentage;
        Self {
            current_size,
            threshold,
            percentage,
            until_compression,
        }
    }

    /// Get warning level for display
    pub fn warning_level(&self) -> CompressionWarningLevel {
        if self.percentage >= 90.0 {
            CompressionWarningLevel::Critical
        } else if self.percentage >= 70.0 {
            CompressionWarningLevel::Warning
        } else {
            CompressionWarningLevel::Normal
        }
    }
}

/// Compression warning level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionWarningLevel {
    Normal,
    Warning,
    Critical,
}

/// Session list item
#[derive(Debug, Clone)]
pub struct SessionListItem {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Local>,
    pub message_count: usize,
}

/// Task info
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Local>,
    pub agent_id: Option<String>,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Pending,
    Completed,
    Failed,
}

/// Output line
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub content: String,
    pub style: OutputStyle,
    pub timestamp: DateTime<Local>,
}

/// Output style
#[derive(Debug, Clone)]
pub enum OutputStyle {
    UserMessage,
    AgentMessage,
    SystemInfo,
    Error,
    Code(String),  // language
    Status(String), // emoji/status text
}

/// Project info
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub path: String,
    pub git_branch: Option<String>,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            path: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            git_branch: None,
        }
    }
}
