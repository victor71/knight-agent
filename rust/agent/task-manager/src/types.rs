//! Task Manager Types
//!
//! Core data types for the task manager module.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Task manager errors
#[derive(Error, Debug)]
pub enum TaskManagerError {
    #[error("Task manager not initialized")]
    NotInitialized,
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Task scheduling failed: {0}")]
    SchedulingFailed(String),
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Dependency error: {0}")]
    DependencyError(String),
    #[error("Circular dependency detected")]
    CircularDependency,
    #[error("Invalid task definition: {0}")]
    InvalidDefinition(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Workflow still running: {0}")]
    WorkflowRunning(String),
}

/// Result type for task manager operations
pub type TaskResult<T> = Result<T, TaskManagerError>;

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TaskType {
    #[default]
    Agent,
    Skill,
    Tool,
    Workflow,
}


/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TaskStatus {
    #[default]
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}


/// Workflow status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum WorkflowStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}


/// Dependency condition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DependencyCondition {
    #[default]
    Success,
    Failed,
    Completed,
}


/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub task_type: TaskType,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub skill: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub inputs: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<Dependency>,
    #[serde(default)]
    pub run_if: Option<String>,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl TaskDefinition {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            task_type: TaskType::Agent,
            agent: None,
            skill: None,
            tool: None,
            inputs: serde_json::Map::new(),
            outputs: Vec::new(),
            depends_on: Vec::new(),
            run_if: None,
            continue_on_error: false,
            retry: None,
            timeout: None,
        }
    }

    pub fn with_agent(mut self, agent: &str) -> Self {
        self.agent = Some(agent.to_string());
        self.task_type = TaskType::Agent;
        self
    }

    pub fn with_skill(mut self, skill: &str) -> Self {
        self.skill = Some(skill.to_string());
        self.task_type = TaskType::Skill;
        self
    }

    pub fn with_tool(mut self, tool: &str) -> Self {
        self.tool = Some(tool.to_string());
        self.task_type = TaskType::Tool;
        self
    }

    pub fn with_inputs(mut self, inputs: serde_json::Map<String, serde_json::Value>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_outputs(mut self, outputs: Vec<String>) -> Self {
        self.outputs = outputs;
        self
    }

    pub fn with_depends_on(mut self, deps: Vec<Dependency>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub task_id: String,
    #[serde(default)]
    pub condition: DependencyCondition,
}

impl Dependency {
    pub fn new(task_id: &str) -> Self {
        Self {
            task_id: task_id.to_string(),
            condition: DependencyCondition::Success,
        }
    }

    pub fn with_condition(mut self, condition: DependencyCondition) -> Self {
        self.condition = condition;
        self
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
    #[serde(default)]
    pub backoff: RetryBackoff,
    #[serde(default)]
    pub retry_on: Vec<String>,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_delay() -> u64 {
    1000
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            delay_ms: 1000,
            backoff: RetryBackoff::Exponential,
            retry_on: Vec::new(),
        }
    }
}

/// Retry backoff strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RetryBackoff {
    Fixed,
    #[default]
    Exponential,
}


/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub variables: serde_json::Map<String, serde_json::Value>,
    pub tasks: Vec<TaskDefinition>,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
}

fn default_max_parallel() -> usize {
    10
}

impl WorkflowDefinition {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            variables: serde_json::Map::new(),
            tasks: Vec::new(),
            retry: None,
            timeout: None,
            max_parallel: 10,
        }
    }

    pub fn with_tasks(mut self, tasks: Vec<TaskDefinition>) -> Self {
        self.tasks = tasks;
        self
    }

    pub fn with_variables(mut self, variables: serde_json::Map<String, serde_json::Value>) -> Self {
        self.variables = variables;
        self
    }
}

/// Task instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub workflow_id: Option<String>,
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub assigned_agent: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub result: Option<TaskExecutionResult>,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub depends_on: Vec<Dependency>,
    #[serde(default)]
    pub dependents: Vec<String>,
    #[serde(default)]
    pub inputs: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub outputs: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub error: Option<ErrorInfo>,
    #[serde(default)]
    pub run_if: Option<String>,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl Task {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            workflow_id: None,
            name: name.to_string(),
            description: description.to_string(),
            task_type: TaskType::Agent,
            status: TaskStatus::Pending,
            progress: 0.0,
            assigned_agent: None,
            started_at: None,
            completed_at: None,
            result: None,
            retry_count: 0,
            depends_on: Vec::new(),
            dependents: Vec::new(),
            inputs: serde_json::Map::new(),
            outputs: serde_json::Map::new(),
            error: None,
            run_if: None,
            continue_on_error: false,
            retry: None,
            timeout: None,
        }
    }

    pub fn from_definition(def: &TaskDefinition, workflow_id: Option<String>) -> Self {
        Self {
            id: def.id.clone(),
            workflow_id,
            name: def.name.clone(),
            description: def.description.clone(),
            task_type: def.task_type,
            status: TaskStatus::Pending,
            progress: 0.0,
            assigned_agent: None,
            started_at: None,
            completed_at: None,
            result: None,
            retry_count: 0,
            depends_on: def.depends_on.clone(),
            dependents: Vec::new(),
            inputs: def.inputs.clone(),
            outputs: serde_json::Map::new(),
            error: None,
            run_if: def.run_if.clone(),
            continue_on_error: def.continue_on_error,
            retry: def.retry.clone(),
            timeout: def.timeout,
        }
    }
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub success: bool,
    #[serde(default)]
    pub outputs: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub execution_time_ms: u64,
}

impl TaskExecutionResult {
    pub fn success(outputs: serde_json::Map<String, serde_json::Value>, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            outputs,
            error: None,
            error_code: None,
            execution_time_ms,
        }
    }

    pub fn failure(error: &str, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            outputs: serde_json::Map::new(),
            error: Some(error.to_string()),
            error_code: None,
            execution_time_ms,
        }
    }
}

/// Error info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: serde_json::Map<String, serde_json::Value>,
}

impl ErrorInfo {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: serde_json::Map::new(),
        }
    }
}

/// Workflow instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: WorkflowStatus,
    #[serde(default)]
    pub variables: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub max_parallel: usize,
}

impl Workflow {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            status: WorkflowStatus::Pending,
            variables: serde_json::Map::new(),
            tasks: Vec::new(),
            progress: 0.0,
            started_at: None,
            completed_at: None,
            error: None,
            max_parallel: 10,
        }
    }

    pub fn from_definition(def: &WorkflowDefinition) -> Self {
        Self {
            id: def.id.clone(),
            name: def.name.clone(),
            description: def.description.clone(),
            status: WorkflowStatus::Pending,
            variables: def.variables.clone(),
            tasks: def.tasks.iter().map(|t| Task::from_definition(t, Some(def.id.clone()))).collect(),
            progress: 0.0,
            started_at: None,
            completed_at: None,
            error: None,
            max_parallel: def.max_parallel,
        }
    }
}

/// Task filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub task_type: Option<TaskType>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub created_after: Option<String>,
    #[serde(default)]
    pub created_before: Option<String>,
}

/// Task update
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskUpdate {
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub result: Option<TaskExecutionResult>,
    #[serde(default)]
    pub error: Option<ErrorInfo>,
    #[serde(default)]
    pub assigned_agent: Option<String>,
}

/// Task history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryEntry {
    pub timestamp: String,
    pub event: String,
    #[serde(default)]
    pub from_status: Option<TaskStatus>,
    pub to_status: TaskStatus,
    #[serde(default)]
    pub details: serde_json::Map<String, serde_json::Value>,
}

impl TaskHistoryEntry {
    pub fn new(event: &str, to_status: TaskStatus) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: event.to_string(),
            from_status: None,
            to_status,
            details: serde_json::Map::new(),
        }
    }
}

/// Workflow log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLogEntry {
    pub timestamp: String,
    pub level: String,
    pub workflow_id: String,
    #[serde(default)]
    pub task_id: Option<String>,
    pub message: String,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl WorkflowLogEntry {
    pub fn new(workflow_id: &str, level: &str, message: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            workflow_id: workflow_id.to_string(),
            task_id: None,
            message: message.to_string(),
            metadata: serde_json::Map::new(),
        }
    }
}

/// Task statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskStatistics {
    #[serde(default)]
    pub total_tasks: u64,
    #[serde(default)]
    pub pending_tasks: u64,
    #[serde(default)]
    pub running_tasks: u64,
    #[serde(default)]
    pub completed_tasks: u64,
    #[serde(default)]
    pub failed_tasks: u64,
    #[serde(default)]
    pub success_rate: f64,
}

/// Dependency info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub task_id: String,
    pub depends_on: Vec<String>,
    pub dependents: Vec<String>,
    pub status: TaskStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_definition_new() {
        let task = TaskDefinition::new("task-1", "Test Task", "A test task");
        assert_eq!(task.id, "task-1");
        assert_eq!(task.name, "Test Task");
        assert_eq!(task.task_type, TaskType::Agent);
    }

    #[test]
    fn test_task_definition_with_skill() {
        let task = TaskDefinition::new("task-1", "Test Task", "A test task")
            .with_skill("code-review");
        assert_eq!(task.skill, Some("code-review".to_string()));
        assert_eq!(task.task_type, TaskType::Skill);
    }

    #[test]
    fn test_dependency_new() {
        let dep = Dependency::new("task-1");
        assert_eq!(dep.task_id, "task-1");
        assert_eq!(dep.condition, DependencyCondition::Success);
    }

    #[test]
    fn test_workflow_definition_new() {
        let wf = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow");
        assert_eq!(wf.id, "wf-1");
        assert!(wf.tasks.is_empty());
    }

    #[test]
    fn test_task_new() {
        let task = Task::new("task-1", "Test Task", "A test task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.progress, 0.0);
    }

    #[test]
    fn test_task_execution_result_success() {
        let result = TaskExecutionResult::success(serde_json::Map::new(), 100);
        assert!(result.success);
        assert_eq!(result.execution_time_ms, 100);
    }

    #[test]
    fn test_task_execution_result_failure() {
        let result = TaskExecutionResult::failure("Error occurred", 50);
        assert!(!result.success);
        assert_eq!(result.error, Some("Error occurred".to_string()));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.backoff, RetryBackoff::Exponential);
    }

    #[test]
    fn test_error_info_new() {
        let error = ErrorInfo::new("ERR_001", "Something went wrong");
        assert_eq!(error.code, "ERR_001");
        assert_eq!(error.message, "Something went wrong");
    }

    #[test]
    fn test_task_filter_default() {
        let filter = TaskFilter::default();
        assert!(filter.workflow_id.is_none());
        assert!(filter.status.is_none());
    }

    #[test]
    fn test_task_update_default() {
        let update = TaskUpdate::default();
        assert!(update.status.is_none());
        assert!(update.progress.is_none());
    }

    #[test]
    fn test_task_statistics_default() {
        let stats = TaskStatistics::default();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_workflow_log_entry_new() {
        let log = WorkflowLogEntry::new("wf-1", "info", "Test message");
        assert_eq!(log.workflow_id, "wf-1");
        assert_eq!(log.level, "info");
        assert_eq!(log.message, "Test message");
    }

    #[test]
    fn test_task_history_entry_new() {
        let entry = TaskHistoryEntry::new("created", TaskStatus::Pending);
        assert_eq!(entry.event, "created");
        assert_eq!(entry.to_status, TaskStatus::Pending);
    }
}
