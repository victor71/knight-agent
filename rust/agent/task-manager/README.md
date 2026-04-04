# Task Manager Module

Design Reference: `docs/03-module-design/agent/task-manager.md`

## Overview

Task Manager handles task lifecycle, workflow orchestration, and dependency resolution:
- Task creation, scheduling, and execution
- Workflow definition and execution
- DAG-based dependency resolution
- Task status tracking and progress monitoring
- Retry configuration and error handling

## Import

```rust
use task_manager::{
    TaskManagerImpl, TaskManagerError, TaskResult, TaskType, TaskStatus,
    WorkflowStatus, DependencyCondition, TaskDefinition, Dependency,
    RetryConfig, RetryBackoff, WorkflowDefinition, Task,
    TaskExecutionResult, ErrorInfo, Workflow, TaskFilter, TaskUpdate,
    TaskHistoryEntry, WorkflowLogEntry, TaskStatistics, DependencyInfo,
};
```

## Core Types

### TaskManagerError
Task manager error enumeration:
- `NotInitialized`: Task manager not initialized
- `TaskNotFound(String)`: Task not found
- `SchedulingFailed(String)`: Task scheduling failed
- `WorkflowNotFound(String)`: Workflow not found
- `DependencyError(String)`: Dependency error
- `CircularDependency`: Circular dependency detected
- `InvalidDefinition(String)`: Invalid task/workflow definition
- `ExecutionFailed(String)`: Execution failed
- `Timeout`: Timeout error
- `WorkflowRunning(String)`: Workflow still running

### TaskType
Task type enumeration:
- `Agent`: Agent-based task
- `Skill`: Skill-based task
- `Tool`: Tool-based task
- `Workflow`: Workflow-based task

### TaskStatus
Task status enumeration:
- `Pending`: Task is pending
- `Ready`: Task is ready to execute
- `Running`: Task is running
- `Completed`: Task completed successfully
- `Failed`: Task failed
- `Cancelled`: Task was cancelled
- `Skipped`: Task was skipped

### WorkflowStatus
Workflow status enumeration:
- `Pending`: Workflow is pending
- `Running`: Workflow is running
- `Completed`: Workflow completed
- `Failed`: Workflow failed
- `Cancelled`: Workflow was cancelled
- `Paused`: Workflow is paused

### DependencyCondition
Condition for dependency satisfaction:
- `Success`: Dependency must complete successfully
- `Failed`: Dependency must fail
- `Completed`: Dependency must complete (success or failure)

## Task Definition

### Creating a Task Definition

```rust
let task = TaskDefinition::new("task-1", "Analyze Code", "Analyze code quality")
    .with_agent("claude")
    .with_timeout(3600)
    .with_retry(RetryConfig::default());
```

### Creating a Task Definition with Dependencies

```rust
let dep = Dependency::new("task-1").with_condition(DependencyCondition::Success);

let task = TaskDefinition::new("task-2", "Review Code", "Review code changes")
    .with_agent("reviewer")
    .with_depends_on(vec![dep]);
```

## Workflow Definition

### Creating a Workflow

```rust
let task1 = TaskDefinition::new("task-1", "Build", "Build the project")
    .with_agent("builder");
let task2 = TaskDefinition::new("task-2", "Test", "Run tests")
    .with_agent("tester")
    .with_depends_on(vec![Dependency::new("task-1")]);
let task3 = TaskDefinition::new("task-3", "Deploy", "Deploy the app")
    .with_agent("deployer")
    .with_depends_on(vec![Dependency::new("task-2")]);

let workflow = WorkflowDefinition::new("wf-1", "CI Pipeline", "CI/CD pipeline")
    .with_tasks(vec![task1, task2, task3])
    .with_max_parallel(10);
```

## Task Manager Operations

### Create Task Manager

```rust
let tm = TaskManagerImpl::new();
```

### Register Workflow

```rust
tm.register_workflow(workflow).await.unwrap();
```

### Start Workflow

```rust
let execution_id = tm.start_workflow("wf-1").await.unwrap();
println!("Workflow started: {}", execution_id);
```

### Get Workflow Status

```rust
let status = tm.get_workflow_status(&execution_id).await.unwrap();
println!("Status: {:?}", status.status);
println!("Progress: {}%", status.progress);
```

### Update Task Status

```rust
let update = TaskUpdate {
    status: Some(TaskStatus::Running),
    progress: Some(50.0),
    ..Default::default()
};
tm.update_task(&execution_id, "task-1", update).await.unwrap();
```

### Cancel Workflow

```rust
tm.cancel_workflow(&execution_id).await.unwrap();
```

### Pause/Resume Workflow

```rust
tm.pause_workflow(&execution_id).await.unwrap();
// ... do something ...
tm.resume_workflow(&execution_id).await.unwrap();
```

### Get Task

```rust
let task = tm.get_task(&execution_id, "task-1").await.unwrap();
println!("Task status: {:?}", task.status);
```

### Get Statistics

```rust
let stats = tm.get_statistics(&execution_id).await.unwrap();
println!("Total: {}, Completed: {}, Failed: {}",
    stats.total_tasks, stats.completed_tasks, stats.failed_tasks);
```

### List Workflows

```rust
let workflows = tm.list_workflows().await;
for wf in workflows {
    println!("- {}: {}", wf.id, wf.name);
}
```

## Complete Example

```rust
use task_manager::{TaskManagerImpl, TaskDefinition, WorkflowDefinition, Dependency, TaskStatus, TaskUpdate};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tm = TaskManagerImpl::new();

    // Create tasks
    let build_task = TaskDefinition::new("build", "Build", "Build the project")
        .with_agent("builder-agent");
    let test_task = TaskDefinition::new("test", "Test", "Run tests")
        .with_agent("tester-agent")
        .with_depends_on(vec![Dependency::new("build")]);
    let deploy_task = TaskDefinition::new("deploy", "Deploy", "Deploy the app")
        .with_agent("deployer-agent")
        .with_depends_on(vec![Dependency::new("test")]);

    // Create workflow
    let workflow = WorkflowDefinition::new("ci", "CI Pipeline", "CI/CD pipeline")
        .with_tasks(vec![build_task, test_task, deploy_task]);

    // Register and start
    tm.register_workflow(workflow).await?;
    let execution_id = tm.start_workflow("ci").await?;

    // Monitor progress
    loop {
        let status = tm.get_workflow_status(&execution_id).await?;
        println!("Progress: {:.1}%", status.progress);

        if status.status == WorkflowStatus::Completed ||
           status.status == WorkflowStatus::Failed {
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}
```

## Error Handling

All operations return `TaskResult<T>` type, using `?` for error propagation:

```rust
match tm.register_workflow(workflow).await {
    Ok(_) => println!("Workflow registered!"),
    Err(TaskManagerError::InvalidDefinition(msg)) => {
        println!("Invalid workflow: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Retry Configuration

```rust
let retry = RetryConfig {
    max_attempts: 3,
    delay_ms: 1000,
    backoff: RetryBackoff::Exponential,
    retry_on: vec!["network_error".to_string()],
};

let task = TaskDefinition::new("task-1", "API Call", "Call external API")
    .with_retry(retry)
    .with_timeout(60);
```
