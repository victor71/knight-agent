//! Task Manager
//!
//! Manages task lifecycle, workflow orchestration, and dependency resolution.
//!
//! Design Reference: docs/03-module-design/agent/task-manager.md

pub mod types;
pub mod manager;

pub use types::{
    TaskManagerError, TaskResult, TaskType, TaskStatus, WorkflowStatus,
    DependencyCondition, TaskDefinition, Dependency, RetryConfig, RetryBackoff,
    WorkflowDefinition, Task, TaskExecutionResult, ErrorInfo, Workflow,
    TaskFilter, TaskUpdate, TaskHistoryEntry, WorkflowLogEntry,
    TaskStatistics, DependencyInfo,
};

pub use manager::{TaskManagerImpl, TaskManagerConfig, TaskEventSender};
