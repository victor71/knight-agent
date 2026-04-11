//! Task Manager
//!
//! Manages task lifecycle, workflow orchestration, and dependency resolution.
//!
//! Design Reference: docs/03-module-design/agent/task-manager.md

pub mod manager;
pub mod types;

pub use types::{
    Dependency, DependencyCondition, DependencyInfo, ErrorInfo, RetryBackoff, RetryConfig, Task,
    TaskDefinition, TaskExecutionResult, TaskFilter, TaskHistoryEntry, TaskManagerError,
    TaskResult, TaskStatistics, TaskStatus, TaskType, TaskUpdate, Workflow, WorkflowDefinition,
    WorkflowLogEntry, WorkflowStatus,
};

pub use manager::{TaskEventSender, TaskManagerConfig, TaskManagerImpl};
