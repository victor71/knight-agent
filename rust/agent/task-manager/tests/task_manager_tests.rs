//! Task Manager Tests
//!
//! Unit tests for the task manager module.

use task_manager::{
    Dependency, DependencyCondition, RetryBackoff, RetryConfig, TaskDefinition, TaskFilter,
    TaskManagerImpl, TaskStatus, TaskType, TaskUpdate, WorkflowDefinition, WorkflowStatus,
};

#[tokio::test]
async fn test_register_workflow() {
    let tm = TaskManagerImpl::new();

    let workflow = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow");
    let result = tm.register_workflow(workflow).await;
    assert!(result.is_ok());
    assert!(tm.has_workflow("wf-1").await);
}

#[tokio::test]
async fn test_register_workflow_with_tasks() {
    let tm = TaskManagerImpl::new();

    let task1 = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let task2 = TaskDefinition::new("task-2", "Task 2", "Second task")
        .with_agent("agent-2")
        .with_depends_on(vec![Dependency::new("task-1")]);

    let workflow = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow")
        .with_tasks(vec![task1, task2]);

    let result = tm.register_workflow(workflow).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_register_duplicate_workflow() {
    let tm = TaskManagerImpl::new();

    let workflow1 = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow");
    tm.register_workflow(workflow1).await.unwrap();

    let workflow2 = WorkflowDefinition::new("wf-1", "Another Workflow", "Duplicate");
    let result = tm.register_workflow(workflow2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_workflow() {
    let tm = TaskManagerImpl::new();

    let workflow = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow");
    tm.register_workflow(workflow).await.unwrap();

    let retrieved = tm.get_workflow("wf-1").await.unwrap();
    assert_eq!(retrieved.id, "wf-1");
    assert_eq!(retrieved.name, "Test Workflow");
}

#[tokio::test]
async fn test_get_nonexistent_workflow() {
    let tm = TaskManagerImpl::new();
    let result = tm.get_workflow("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_workflows() {
    let tm = TaskManagerImpl::new();

    let workflow1 = WorkflowDefinition::new("wf-1", "Workflow 1", "First");
    let workflow2 = WorkflowDefinition::new("wf-2", "Workflow 2", "Second");

    tm.register_workflow(workflow1).await.unwrap();
    tm.register_workflow(workflow2).await.unwrap();

    let workflows = tm.list_workflows().await;
    assert_eq!(workflows.len(), 2);
}

#[tokio::test]
async fn test_unregister_workflow() {
    let tm = TaskManagerImpl::new();

    let workflow = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow");
    tm.register_workflow(workflow).await.unwrap();

    let result = tm.unregister_workflow("wf-1").await;
    assert!(result.is_ok());
    assert!(!tm.has_workflow("wf-1").await);
}

#[tokio::test]
async fn test_start_workflow() {
    let tm = TaskManagerImpl::new();

    let task1 = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let task2 = TaskDefinition::new("task-2", "Task 2", "Second task")
        .with_agent("agent-2")
        .with_depends_on(vec![Dependency::new("task-1")]);

    let workflow = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow")
        .with_tasks(vec![task1, task2]);

    tm.register_workflow(workflow).await.unwrap();

    let execution_id = tm.start_workflow("wf-1").await.unwrap();
    assert!(!execution_id.is_empty());

    let status = tm.get_workflow_status(&execution_id).await.unwrap();
    assert_eq!(status.status, WorkflowStatus::Running);
    assert!(status.started_at.is_some());
}

#[tokio::test]
async fn test_start_workflow_no_dependencies() {
    let tm = TaskManagerImpl::new();

    let task1 = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let task2 = TaskDefinition::new("task-2", "Task 2", "Second task").with_agent("agent-2");

    let workflow = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow")
        .with_tasks(vec![task1, task2]);

    tm.register_workflow(workflow).await.unwrap();

    let execution_id = tm.start_workflow("wf-1").await.unwrap();
    let status = tm.get_workflow_status(&execution_id).await.unwrap();

    // Both tasks should be ready since there are no dependencies
    let ready_count = status
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Ready)
        .count();
    assert_eq!(ready_count, 2);
}

#[tokio::test]
async fn test_cancel_workflow() {
    let tm = TaskManagerImpl::new();

    let task = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let workflow =
        WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow").with_tasks(vec![task]);

    tm.register_workflow(workflow).await.unwrap();

    let execution_id = tm.start_workflow("wf-1").await.unwrap();
    let result = tm.cancel_workflow(&execution_id).await;

    assert!(result.is_ok());
    let status = tm.get_workflow_status(&execution_id).await.unwrap();
    assert_eq!(status.status, WorkflowStatus::Cancelled);
    assert!(status.completed_at.is_some());
}

#[tokio::test]
async fn test_pause_and_resume_workflow() {
    let tm = TaskManagerImpl::new();

    let task = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let workflow =
        WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow").with_tasks(vec![task]);

    tm.register_workflow(workflow).await.unwrap();

    let execution_id = tm.start_workflow("wf-1").await.unwrap();

    // Pause
    let pause_result = tm.pause_workflow(&execution_id).await;
    assert!(pause_result.is_ok());

    let status = tm.get_workflow_status(&execution_id).await.unwrap();
    assert_eq!(status.status, WorkflowStatus::Paused);

    // Resume
    let resume_result = tm.resume_workflow(&execution_id).await;
    assert!(resume_result.is_ok());

    let status = tm.get_workflow_status(&execution_id).await.unwrap();
    assert_eq!(status.status, WorkflowStatus::Running);
}

#[tokio::test]
async fn test_update_task_status() {
    let tm = TaskManagerImpl::new();

    let task = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let workflow =
        WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow").with_tasks(vec![task]);

    tm.register_workflow(workflow).await.unwrap();

    let execution_id = tm.start_workflow("wf-1").await.unwrap();

    // Update task to running
    let update = TaskUpdate {
        status: Some(TaskStatus::Running),
        progress: None,
        result: None,
        error: None,
        assigned_agent: None,
    };

    let result = tm.update_task(&execution_id, "task-1", update).await;
    assert!(result.is_ok());

    let task = tm.get_task(&execution_id, "task-1").await.unwrap();
    assert_eq!(task.status, TaskStatus::Running);
}

#[tokio::test]
async fn test_get_statistics() {
    let tm = TaskManagerImpl::new();

    let task1 = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let task2 = TaskDefinition::new("task-2", "Task 2", "Second task").with_agent("agent-2");

    let workflow = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow")
        .with_tasks(vec![task1, task2]);

    tm.register_workflow(workflow).await.unwrap();

    let execution_id = tm.start_workflow("wf-1").await.unwrap();

    let stats = tm.get_statistics(&execution_id).await.unwrap();
    assert_eq!(stats.total_tasks, 2);
    assert_eq!(stats.pending_tasks, 0); // Tasks are set to Ready since they have no dependencies
    assert_eq!(stats.running_tasks, 0);
    assert_eq!(stats.completed_tasks, 0);
}

#[tokio::test]
async fn test_active_workflow_count() {
    let tm = TaskManagerImpl::new();
    assert_eq!(tm.active_workflow_count().await, 0);

    let task = TaskDefinition::new("task-1", "Task 1", "First task").with_agent("agent-1");

    let workflow =
        WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow").with_tasks(vec![task]);

    tm.register_workflow(workflow).await.unwrap();
    tm.start_workflow("wf-1").await.unwrap();

    assert_eq!(tm.active_workflow_count().await, 1);
}

// Type tests

#[test]
fn test_task_definition_new() {
    let task = TaskDefinition::new("task-1", "Test Task", "A test task");
    assert_eq!(task.id, "task-1");
    assert_eq!(task.name, "Test Task");
    assert_eq!(task.task_type, TaskType::Agent);
}

#[test]
fn test_task_definition_with_agent() {
    let task = TaskDefinition::new("task-1", "Test Task", "A test task").with_agent("agent-1");
    assert_eq!(task.agent, Some("agent-1".to_string()));
    assert_eq!(task.task_type, TaskType::Agent);
}

#[test]
fn test_task_definition_with_skill() {
    let task = TaskDefinition::new("task-1", "Test Task", "A test task").with_skill("code-review");
    assert_eq!(task.skill, Some("code-review".to_string()));
    assert_eq!(task.task_type, TaskType::Skill);
}

#[test]
fn test_task_definition_with_tool() {
    let task = TaskDefinition::new("task-1", "Test Task", "A test task").with_tool("read_file");
    assert_eq!(task.tool, Some("read_file".to_string()));
    assert_eq!(task.task_type, TaskType::Tool);
}

#[test]
fn test_task_definition_with_retry() {
    let retry = RetryConfig {
        max_attempts: 5,
        delay_ms: 2000,
        backoff: RetryBackoff::Exponential,
        retry_on: vec!["error".to_string()],
    };
    let task = TaskDefinition::new("task-1", "Test Task", "A test task").with_retry(retry);
    assert!(task.retry.is_some());
    assert_eq!(task.retry.as_ref().unwrap().max_attempts, 5);
}

#[test]
fn test_task_definition_with_timeout() {
    let task = TaskDefinition::new("task-1", "Test Task", "A test task").with_timeout(3600);
    assert_eq!(task.timeout, Some(3600));
}

#[test]
fn test_dependency_new() {
    let dep = Dependency::new("task-1");
    assert_eq!(dep.task_id, "task-1");
    assert_eq!(dep.condition, DependencyCondition::Success);
}

#[test]
fn test_dependency_with_condition() {
    let dep = Dependency::new("task-1").with_condition(DependencyCondition::Failed);
    assert_eq!(dep.condition, DependencyCondition::Failed);
}

#[test]
fn test_workflow_definition_new() {
    let wf = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow");
    assert_eq!(wf.id, "wf-1");
    assert_eq!(wf.name, "Test Workflow");
    assert!(wf.tasks.is_empty());
    assert_eq!(wf.max_parallel, 10);
}

#[test]
fn test_workflow_definition_with_tasks() {
    let task1 = TaskDefinition::new("task-1", "Task 1", "First task");
    let task2 = TaskDefinition::new("task-2", "Task 2", "Second task");

    let wf = WorkflowDefinition::new("wf-1", "Test Workflow", "A test workflow")
        .with_tasks(vec![task1, task2]);

    assert_eq!(wf.tasks.len(), 2);
}

#[test]
fn test_retry_config_default() {
    let config = RetryConfig::default();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.delay_ms, 1000);
    assert_eq!(config.backoff, RetryBackoff::Exponential);
    assert!(config.retry_on.is_empty());
}

#[test]
fn test_task_filter_default() {
    let filter = TaskFilter::default();
    assert!(filter.workflow_id.is_none());
    assert!(filter.status.is_none());
    assert!(filter.task_type.is_none());
}

#[test]
fn test_task_update_default() {
    let update = TaskUpdate::default();
    assert!(update.status.is_none());
    assert!(update.progress.is_none());
    assert!(update.result.is_none());
}
