//! Task Manager Implementation
//!
//! Manages task lifecycle, workflow orchestration, and dependency resolution.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock as AsyncRwLock;
use tracing::info;

use crate::types::*;

/// Task Manager implementation
pub struct TaskManagerImpl {
    /// Workflow definitions (workflow_id -> WorkflowDefinition)
    workflows: Arc<AsyncRwLock<HashMap<String, WorkflowDefinition>>>,
    /// Active workflows (workflow_id -> Workflow)
    active_workflows: Arc<AsyncRwLock<HashMap<String, Workflow>>>,
    /// Task queue for pending tasks
    task_queue: Arc<AsyncRwLock<VecDeque<String>>>,
    /// Configuration
    config: Arc<Mutex<TaskManagerConfig>>,
    /// Event sender for task events
    event_sender: Arc<Mutex<Option<Box<dyn TaskEventSender + Send + Sync>>>>,
}

impl TaskManagerImpl {
    /// Create a new task manager
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(AsyncRwLock::new(HashMap::new())),
            active_workflows: Arc::new(AsyncRwLock::new(HashMap::new())),
            task_queue: Arc::new(AsyncRwLock::new(VecDeque::new())),
            config: Arc::new(Mutex::new(TaskManagerConfig::default())),
            event_sender: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with custom config
    pub fn with_config(config: TaskManagerConfig) -> Self {
        Self {
            workflows: Arc::new(AsyncRwLock::new(HashMap::new())),
            active_workflows: Arc::new(AsyncRwLock::new(HashMap::new())),
            task_queue: Arc::new(AsyncRwLock::new(VecDeque::new())),
            config: Arc::new(Mutex::new(config)),
            event_sender: Arc::new(Mutex::new(None)),
        }
    }

    // ========== Workflow Management ==========

    /// Register a workflow definition
    pub async fn register_workflow(&self, workflow: WorkflowDefinition) -> TaskResult<()> {
        let workflows = self.workflows.read().await;
        let workflow_id = workflow.id.clone();

        // Check if workflow already exists
        if workflows.contains_key(&workflow_id) {
            return Err(TaskManagerError::InvalidDefinition(
                format!("Workflow {} already registered", workflow_id),
            ));
        }
        drop(workflows);

        // Validate workflow
        self.validate_workflow(&workflow)?;

        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id.clone(), workflow);
        info!("Registered workflow: {}", workflow_id);
        Ok(())
    }

    /// Get a workflow definition
    pub async fn get_workflow(&self, workflow_id: &str) -> TaskResult<WorkflowDefinition> {
        let workflows = self.workflows.read().await;
        workflows
            .get(workflow_id)
            .cloned()
            .ok_or_else(|| TaskManagerError::WorkflowNotFound(workflow_id.to_string()))
    }

    /// List all workflow definitions
    pub async fn list_workflows(&self) -> Vec<WorkflowDefinition> {
        let workflows = self.workflows.read().await;
        workflows.values().cloned().collect()
    }

    /// Unregister a workflow definition
    pub async fn unregister_workflow(&self, workflow_id: &str) -> TaskResult<()> {
        let mut workflows = self.workflows.write().await;

        // Check if workflow is running
        let active_workflows = self.active_workflows.read().await;
        if active_workflows.contains_key(workflow_id) {
            return Err(TaskManagerError::WorkflowRunning(
                format!("Workflow {} is still running", workflow_id),
            ));
        }

        workflows
            .remove(workflow_id)
            .ok_or_else(|| TaskManagerError::WorkflowNotFound(workflow_id.to_string()))?;

        info!("Unregistered workflow: {}", workflow_id);
        Ok(())
    }

    // ========== Workflow Execution ==========

    /// Start a workflow
    pub async fn start_workflow(&self, workflow_id: &str) -> TaskResult<String> {
        // Get workflow definition
        let workflow_def = self.get_workflow(workflow_id).await?;

        // Create workflow instance
        let mut workflow = Workflow::from_definition(&workflow_def);

        // Build dependency graph and determine initial ready tasks
        let ready_task_ids = self.compute_ready_tasks(&workflow)?;

        // Update workflow status
        workflow.status = WorkflowStatus::Running;
        workflow.started_at = Some(chrono::Utc::now().to_rfc3339());

        // Mark ready tasks as Ready and queue them
        for task in &mut workflow.tasks {
            if ready_task_ids.contains(&task.id) {
                task.status = TaskStatus::Ready;
            }
        }

        // Add to active workflows
        let execution_id = workflow.id.clone();
        {
            let mut active = self.active_workflows.write().await;
            active.insert(execution_id.clone(), workflow);
        }

        // Queue ready tasks
        {
            let mut queue = self.task_queue.write().await;
            for task_id in ready_task_ids {
                queue.push_back(task_id);
            }
        }

        info!("Started workflow: {} (execution: {})", workflow_id, execution_id);
        Ok(execution_id)
    }

    /// Get workflow execution status
    pub async fn get_workflow_status(&self, execution_id: &str) -> TaskResult<Workflow> {
        let active_workflows = self.active_workflows.read().await;
        active_workflows
            .get(execution_id)
            .cloned()
            .ok_or_else(|| TaskManagerError::WorkflowNotFound(execution_id.to_string()))
    }

    /// Cancel a workflow execution
    pub async fn cancel_workflow(&self, execution_id: &str) -> TaskResult<()> {
        let mut active_workflows = self.active_workflows.write().await;

        if let Some(workflow) = active_workflows.get_mut(execution_id) {
            workflow.status = WorkflowStatus::Cancelled;
            workflow.completed_at = Some(chrono::Utc::now().to_rfc3339());

            // Cancel all running tasks
            for task in &mut workflow.tasks {
                if task.status == TaskStatus::Running || task.status == TaskStatus::Ready {
                    task.status = TaskStatus::Cancelled;
                }
            }

            info!("Cancelled workflow: {}", execution_id);
            Ok(())
        } else {
            Err(TaskManagerError::WorkflowNotFound(execution_id.to_string()))
        }
    }

    /// Pause a workflow execution
    pub async fn pause_workflow(&self, execution_id: &str) -> TaskResult<()> {
        let mut active_workflows = self.active_workflows.write().await;

        if let Some(workflow) = active_workflows.get_mut(execution_id) {
            if workflow.status != WorkflowStatus::Running {
                return Err(TaskManagerError::ExecutionFailed(
                    "Workflow is not running".to_string(),
                ));
            }
            workflow.status = WorkflowStatus::Paused;
            info!("Paused workflow: {}", execution_id);
            Ok(())
        } else {
            Err(TaskManagerError::WorkflowNotFound(execution_id.to_string()))
        }
    }

    /// Resume a paused workflow
    pub async fn resume_workflow(&self, execution_id: &str) -> TaskResult<()> {
        let mut active_workflows = self.active_workflows.write().await;

        if let Some(workflow) = active_workflows.get_mut(execution_id) {
            if workflow.status != WorkflowStatus::Paused {
                return Err(TaskManagerError::ExecutionFailed(
                    "Workflow is not paused".to_string(),
                ));
            }
            workflow.status = WorkflowStatus::Running;

            // Re-queue ready tasks
            let ready_task_ids: Vec<String> = workflow
                .tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Ready)
                .map(|t| t.id.clone())
                .collect();

            drop(active_workflows);

            let mut queue = self.task_queue.write().await;
            for task_id in ready_task_ids {
                queue.push_back(task_id);
            }

            info!("Resumed workflow: {}", execution_id);
            Ok(())
        } else {
            Err(TaskManagerError::WorkflowNotFound(execution_id.to_string()))
        }
    }

    // ========== Task Management ==========

    /// Create a standalone task
    pub async fn create_task(&self, task_def: TaskDefinition) -> TaskResult<Task> {
        // Validate task
        if task_def.name.is_empty() {
            return Err(TaskManagerError::InvalidDefinition("Task name is required".to_string()));
        }

        let task = Task::from_definition(&task_def, None);
        Ok(task)
    }

    /// Get a task from a workflow execution
    pub async fn get_task(&self, execution_id: &str, task_id: &str) -> TaskResult<Task> {
        let active_workflows = self.active_workflows.read().await;

        if let Some(workflow) = active_workflows.get(execution_id) {
            workflow
                .tasks
                .iter()
                .find(|t| t.id == task_id)
                .cloned()
                .ok_or_else(|| TaskManagerError::TaskNotFound(task_id.to_string()))
        } else {
            Err(TaskManagerError::WorkflowNotFound(execution_id.to_string()))
        }
    }

    /// Update task status
    pub async fn update_task(
        &self,
        execution_id: &str,
        task_id: &str,
        update: TaskUpdate,
    ) -> TaskResult<()> {
        let mut active_workflows = self.active_workflows.write().await;

        if let Some(workflow) = active_workflows.get_mut(execution_id) {
            if let Some(task) = workflow.tasks.iter_mut().find(|t| t.id == task_id) {
                if let Some(status) = update.status {
                    task.status = status;
                    if status == TaskStatus::Running {
                        task.started_at = Some(chrono::Utc::now().to_rfc3339());
                    } else if status == TaskStatus::Completed || status == TaskStatus::Failed {
                        task.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                }
                if let Some(progress) = update.progress {
                    task.progress = progress;
                }
                if let Some(result) = update.result {
                    task.result = Some(result);
                }
                if let Some(error) = update.error {
                    task.error = Some(error);
                }
                if let Some(agent) = update.assigned_agent {
                    task.assigned_agent = Some(agent);
                }

                // Check if task completion triggers dependent tasks
                if task.status == TaskStatus::Completed || task.status == TaskStatus::Failed {
                    self.process_task_completion(workflow, task_id).await?;
                }

                Ok(())
            } else {
                Err(TaskManagerError::TaskNotFound(task_id.to_string()))
            }
        } else {
            Err(TaskManagerError::WorkflowNotFound(execution_id.to_string()))
        }
    }

    /// Get next pending task from queue
    pub async fn get_next_task(&self) -> Option<(String, Task)> {
        let mut queue = self.task_queue.write().await;

        while let Some(task_id) = queue.pop_front() {
            // Find task in active workflows
            let active_workflows = self.active_workflows.read().await;
            for workflow in active_workflows.values() {
                if let Some(task) = workflow.tasks.iter().find(|t| t.id == task_id) {
                    if task.status == TaskStatus::Ready || task.status == TaskStatus::Pending {
                        return Some((workflow.id.clone(), task.clone()));
                    }
                }
            }
        }
        None
    }

    // ========== Dependency Resolution ==========

    /// Validate workflow for circular dependencies
    fn validate_workflow(&self, workflow: &WorkflowDefinition) -> TaskResult<()> {
        let task_ids: HashSet<&str> = workflow.tasks.iter().map(|t| t.id.as_str()).collect();

        // Check that all dependencies reference valid tasks
        for task in &workflow.tasks {
            for dep in &task.depends_on {
                if !task_ids.contains(dep.task_id.as_str()) {
                    return Err(TaskManagerError::DependencyError(format!(
                        "Task {} depends on unknown task {}",
                        task.id, dep.task_id
                    )));
                }
            }
        }

        // Check for circular dependencies
        if self.has_circular_dependency(workflow)? {
            return Err(TaskManagerError::CircularDependency);
        }

        Ok(())
    }

    /// Check for circular dependencies using DFS
    fn has_circular_dependency(&self, workflow: &WorkflowDefinition) -> TaskResult<bool> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for task in &workflow.tasks {
            if !visited.contains(&task.id) {
                if self.detect_cycle(workflow, &task.id, &mut visited, &mut recursion_stack)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn detect_cycle(
        &self,
        workflow: &WorkflowDefinition,
        task_id: &str,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> TaskResult<bool> {
        visited.insert(task_id.to_string());
        recursion_stack.insert(task_id.to_string());

        if let Some(task) = workflow.tasks.iter().find(|t| t.id == task_id) {
            for dep in &task.depends_on {
                if !visited.contains(&dep.task_id) {
                    if self.detect_cycle(workflow, &dep.task_id, visited, recursion_stack)? {
                        return Ok(true);
                    }
                } else if recursion_stack.contains(&dep.task_id) {
                    return Ok(true);
                }
            }
        }

        recursion_stack.remove(task_id);
        Ok(false)
    }

    /// Compute which tasks are ready to execute
    fn compute_ready_tasks(&self, workflow: &Workflow) -> TaskResult<Vec<String>> {
        let mut ready = Vec::new();

        for task in &workflow.tasks {
            if task.status != TaskStatus::Pending {
                continue;
            }

            // Check if all dependencies are satisfied
            let all_deps_satisfied = task.depends_on.iter().all(|dep| {
                workflow
                    .tasks
                    .iter()
                    .find(|t| t.id == dep.task_id)
                    .map(|t| match dep.condition {
                        DependencyCondition::Success => t.status == TaskStatus::Completed,
                        DependencyCondition::Failed => t.status == TaskStatus::Failed,
                        DependencyCondition::Completed => {
                            t.status == TaskStatus::Completed || t.status == TaskStatus::Failed
                        }
                    })
                    .unwrap_or(false)
            });

            if all_deps_satisfied {
                ready.push(task.id.clone());
            }
        }

        Ok(ready)
    }

    /// Process task completion and update dependent tasks
    async fn process_task_completion(&self, workflow: &mut Workflow, completed_task_id: &str) -> TaskResult<()> {
        // Build a map of task_id -> status for dependency checks
        let task_statuses: HashMap<String, TaskStatus> = workflow
            .tasks
            .iter()
            .map(|t| (t.id.clone(), t.status))
            .collect();

        // Find tasks that depend on the completed task
        let dependent_ids: Vec<String> = workflow
            .tasks
            .iter()
            .filter(|t| t.depends_on.iter().any(|d| d.task_id == completed_task_id))
            .map(|t| t.id.clone())
            .collect();

        // Update task statuses using the status map
        for task in &mut workflow.tasks {
            if dependent_ids.contains(&task.id) {
                // Check if all dependencies for this task are now satisfied
                let all_deps_satisfied = task.depends_on.iter().all(|dep| {
                    task_statuses
                        .get(&dep.task_id)
                        .map(|status| match dep.condition {
                            DependencyCondition::Success => *status == TaskStatus::Completed,
                            DependencyCondition::Failed => *status == TaskStatus::Failed,
                            DependencyCondition::Completed => {
                                *status == TaskStatus::Completed || *status == TaskStatus::Failed
                            }
                        })
                        .unwrap_or(false)
                });

                if all_deps_satisfied {
                    task.status = TaskStatus::Ready;
                }
            }
        }

        // Update workflow progress
        let completed_count = workflow.tasks.iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let total_count = workflow.tasks.len();
        workflow.progress = (completed_count as f64 / total_count as f64) * 100.0;

        // Check if workflow is complete
        let running_count = workflow.tasks.iter()
            .filter(|t| t.status == TaskStatus::Running || t.status == TaskStatus::Ready || t.status == TaskStatus::Pending)
            .count();

        if running_count == 0 {
            let failed_count = workflow.tasks.iter()
                .filter(|t| t.status == TaskStatus::Failed)
                .count();

            workflow.status = if failed_count > 0 {
                WorkflowStatus::Failed
            } else {
                WorkflowStatus::Completed
            };
            workflow.completed_at = Some(chrono::Utc::now().to_rfc3339());
            info!("Workflow {} completed with status {:?}", workflow.id, workflow.status);
        }

        Ok(())
    }

    // ========== Statistics ==========

    /// Get task statistics
    pub async fn get_statistics(&self, execution_id: &str) -> TaskResult<TaskStatistics> {
        let active_workflows = self.active_workflows.read().await;

        if let Some(workflow) = active_workflows.get(execution_id) {
            let total = workflow.tasks.len() as u64;
            let pending = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count() as u64;
            let running = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Running).count() as u64;
            let completed = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count() as u64;
            let failed = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Failed).count() as u64;

            Ok(TaskStatistics {
                total_tasks: total,
                pending_tasks: pending,
                running_tasks: running,
                completed_tasks: completed,
                failed_tasks: failed,
                success_rate: if total > 0 {
                    (completed as f64 / total as f64) * 100.0
                } else {
                    0.0
                },
            })
        } else {
            Err(TaskManagerError::WorkflowNotFound(execution_id.to_string()))
        }
    }

    /// Get pending task count
    pub async fn pending_count(&self) -> usize {
        let queue = self.task_queue.read().await;
        queue.len()
    }

    /// Check if workflow exists
    pub async fn has_workflow(&self, workflow_id: &str) -> bool {
        let workflows = self.workflows.read().await;
        workflows.contains_key(workflow_id)
    }

    /// Get active workflow count
    pub async fn active_workflow_count(&self) -> usize {
        let active = self.active_workflows.read().await;
        active.len()
    }
}

impl Default for TaskManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Task Manager configuration
#[derive(Debug, Clone)]
pub struct TaskManagerConfig {
    pub max_concurrent_tasks: usize,
    pub default_timeout: u64,
    pub enable_history: bool,
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            default_timeout: 3600,
            enable_history: true,
        }
    }
}

/// Trait for task event sender
pub trait TaskEventSender: Send + Sync {
    fn send_task_started(&self, task_id: &str);
    fn send_task_completed(&self, task_id: &str, result: &TaskExecutionResult);
    fn send_task_failed(&self, task_id: &str, error: &str);
    fn send_workflow_started(&self, workflow_id: &str);
    fn send_workflow_completed(&self, workflow_id: &str, status: WorkflowStatus);
}
