//! Skill Engine Manager
//!
//! Manages skill registration, discovery, and execution.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info};

use crate::types::*;

/// Skill engine implementation
pub struct SkillEngineImpl {
    /// Registered skills (skill_id -> SkillDefinition)
    skills: Arc<AsyncRwLock<HashMap<String, SkillDefinition>>>,
    /// Skill definitions by category
    categories: Arc<AsyncRwLock<HashMap<String, Vec<String>>>>,
    /// Execution history
    execution_history: Arc<AsyncRwLock<Vec<SkillExecutionResult>>>,
}

impl SkillEngineImpl {
    /// Create a new skill engine
    pub fn new() -> Self {
        Self {
            skills: Arc::new(AsyncRwLock::new(HashMap::new())),
            categories: Arc::new(AsyncRwLock::new(HashMap::new())),
            execution_history: Arc::new(AsyncRwLock::new(Vec::new())),
        }
    }

    /// Register a new skill
    pub async fn register_skill(&self, skill: SkillDefinition) -> SkillResult<()> {
        let mut skills = self.skills.write().await;
        let skill_id = skill.id.clone();

        if skills.contains_key(&skill_id) {
            return Err(SkillEngineError::AlreadyRegistered(skill_id));
        }

        // Validate skill definition
        if skill.id.is_empty() || skill.name.is_empty() {
            return Err(SkillEngineError::InvalidDefinition(
                "Skill must have non-empty id and name".to_string(),
            ));
        }

        // Add to skills map
        skills.insert(skill_id.clone(), skill.clone());

        // Add to category index
        if let Some(category) = &skill.category {
            let mut categories = self.categories.write().await;
            categories
                .entry(category.clone())
                .or_insert_with(Vec::new)
                .push(skill_id.clone());
        }

        info!("Registered skill: {}", skill_id);
        Ok(())
    }

    /// Get a skill by ID
    pub async fn get_skill(&self, skill_id: &str) -> SkillResult<SkillDefinition> {
        let skills = self.skills.read().await;
        skills
            .get(skill_id)
            .cloned()
            .ok_or_else(|| SkillEngineError::SkillNotFound(skill_id.to_string()))
    }

    /// List all skills
    pub async fn list_skills(&self) -> Vec<SkillInfo> {
        let skills = self.skills.read().await;
        skills.values().map(SkillInfo::from).collect()
    }

    /// List skills by category
    pub async fn list_skills_by_category(&self, category: &str) -> SkillResult<Vec<SkillInfo>> {
        let categories = self.categories.read().await;
        let skill_ids = categories
            .get(category)
            .ok_or_else(|| SkillEngineError::SkillNotFound(category.to_string()))?;

        let skills = self.skills.read().await;
        let result: Vec<SkillInfo> = skill_ids
            .iter()
            .filter_map(|id| skills.get(id).map(SkillInfo::from))
            .collect();

        Ok(result)
    }

    /// List all categories
    pub async fn list_categories(&self) -> Vec<String> {
        let categories = self.categories.read().await;
        categories.keys().cloned().collect()
    }

    /// Unregister a skill
    pub async fn unregister_skill(&self, skill_id: &str) -> SkillResult<()> {
        let mut skills = self.skills.write().await;

        if let Some(skill) = skills.remove(skill_id) {
            // Remove from category index
            if let Some(category) = &skill.category {
                let mut categories = self.categories.write().await;
                if let Some(ids) = categories.get_mut(category) {
                    ids.retain(|id| id != skill_id);
                    if ids.is_empty() {
                        categories.remove(category);
                    }
                }
            }
            info!("Unregistered skill: {}", skill_id);
            Ok(())
        } else {
            Err(SkillEngineError::SkillNotFound(skill_id.to_string()))
        }
    }

    /// Update a skill
    pub async fn update_skill(&self, skill: SkillDefinition) -> SkillResult<()> {
        let mut skills = self.skills.write().await;
        let skill_id = skill.id.clone();

        // Check if skill exists
        if !skills.contains_key(&skill_id) {
            return Err(SkillEngineError::SkillNotFound(skill_id));
        }

        // Remove from old category if exists
        if let Some(old_skill) = skills.get(&skill_id) {
            if let Some(old_category) = &old_skill.category {
                let mut categories = self.categories.write().await;
                if let Some(ids) = categories.get_mut(old_category) {
                    ids.retain(|id| id != &skill_id);
                }
            }
        }

        // Insert updated skill
        skills.insert(skill_id.clone(), skill.clone());

        // Add to new category
        if let Some(category) = &skill.category {
            let mut categories = self.categories.write().await;
            categories
                .entry(category.clone())
                .or_insert_with(Vec::new)
                .push(skill_id.clone());
        }

        debug!("Updated skill: {}", skill_id);
        Ok(())
    }

    /// Execute a skill
    pub async fn execute_skill(
        &self,
        skill_id: &str,
        context: &SkillContext,
        parameters: serde_json::Map<String, serde_json::Value>,
    ) -> SkillResult<SkillExecutionResult> {
        let start_time = std::time::Instant::now();

        // Get skill definition
        let skill = {
            let skills = self.skills.read().await;
            skills
                .get(skill_id)
                .cloned()
                .ok_or_else(|| SkillEngineError::SkillNotFound(skill_id.to_string()))?
        };

        if !skill.enabled {
            return Err(SkillEngineError::ExecutionFailed(format!(
                "Skill {} is disabled",
                skill_id
            )));
        }

        // Validate required parameters
        for param in &skill.parameters {
            if param.required && !parameters.contains_key(&param.name) && param.default_value.is_none() {
                return Err(SkillEngineError::InvalidDefinition(format!(
                    "Missing required parameter: {}",
                    param.name
                )));
            }
        }

        // Execute steps
        let mut steps_completed = Vec::new();

        for step in &skill.steps {
            // Check condition if present
            if let Some(condition) = &step.condition {
                if !self.evaluate_condition(condition, context) {
                    debug!("Skipping step {} due to condition", step.id);
                    continue;
                }
            }

            // Execute step based on type
            match step.step_type {
                StepType::Action => {
                    debug!("Executing action step: {}", step.name);
                    steps_completed.push(step.id.clone());
                }
                StepType::Skill => {
                    debug!("Executing skill step: {}", step.name);
                    steps_completed.push(step.id.clone());
                }
                StepType::Agent => {
                    debug!("Executing agent step: {}", step.name);
                    steps_completed.push(step.id.clone());
                }
                StepType::Condition => {
                    debug!("Evaluating condition step: {}", step.name);
                    steps_completed.push(step.id.clone());
                }
            }
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        let result = SkillExecutionResult::success(
            skill_id,
            serde_json::json!({ "steps_completed": steps_completed.len() }),
            execution_time_ms,
        )
        .with_steps_completed(steps_completed);

        // Store execution result
        {
            let mut history = self.execution_history.write().await;
            history.push(result.clone());
        }

        Ok(result)
    }

    /// Evaluate a condition expression
    fn evaluate_condition(&self, condition: &str, context: &SkillContext) -> bool {
        // Simple condition evaluation - check if variable exists and is truthy
        if let Some(value) = context.variables.get(condition) {
            return value.as_bool().unwrap_or(false);
        }
        false
    }

    /// Create an execution plan for a task
    pub async fn create_execution_plan(&self, task: &str) -> SkillResult<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();

        // Find matching skills based on triggers
        let skills = self.skills.read().await;

        for skill in skills.values() {
            for trigger in &skill.triggers {
                match trigger.trigger_type {
                    TriggerType::Keyword => {
                        if let Some(pattern) = &trigger.pattern {
                            if task.contains(pattern) {
                                plan.steps.push(PlannedStep {
                                    sequence: plan.steps.len(),
                                    skill_id: skill.id.clone(),
                                    parameters: serde_json::Map::new(),
                                });
                            }
                        }
                    }
                    TriggerType::Event => {
                        // Event-based triggers would be handled by event system
                    }
                    TriggerType::Timer => {
                        // Timer-based triggers would be handled by timer system
                    }
                    TriggerType::FileChange => {
                        // File change triggers would be handled by file watcher
                    }
                }
            }
        }

        Ok(plan)
    }

    /// Execute a pipeline
    pub async fn execute_pipeline(
        &self,
        pipeline: &Pipeline,
        context: &SkillContext,
    ) -> SkillResult<SkillExecutionResult> {
        let start_time = std::time::Instant::now();
        let mut steps_completed = Vec::new();

        for step in &pipeline.steps {
            // Check condition
            if let Some(condition) = &step.condition {
                if !self.evaluate_condition(condition, context) {
                    continue;
                }
            }

            // Execute skill
            match self
                .execute_skill(&step.skill_id, context, step.parameters.clone())
                .await
            {
                Ok(result) => {
                    steps_completed.push(step.skill_id.clone());
                    if !result.success {
                        return Err(SkillEngineError::PipelineError(format!(
                            "Skill {} failed in pipeline",
                            step.skill_id
                        )));
                    }
                }
                Err(e) => {
                    return Err(SkillEngineError::PipelineError(format!(
                        "Pipeline error at skill {}: {}",
                        step.skill_id, e
                    )));
                }
            }
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(SkillExecutionResult::success(
            &pipeline.id,
            serde_json::json!({ "pipeline_completed": true }),
            execution_time_ms,
        )
        .with_steps_completed(steps_completed))
    }

    /// Get execution history
    pub async fn get_execution_history(&self, limit: usize) -> Vec<SkillExecutionResult> {
        let history = self.execution_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Clear execution history
    pub async fn clear_history(&self) {
        let mut history = self.execution_history.write().await;
        history.clear();
    }

    /// Get skill count
    pub async fn skill_count(&self) -> usize {
        let skills = self.skills.read().await;
        skills.len()
    }

    /// Check if skill exists
    pub async fn has_skill(&self, skill_id: &str) -> bool {
        let skills = self.skills.read().await;
        skills.contains_key(skill_id)
    }
}

impl Default for SkillEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}
