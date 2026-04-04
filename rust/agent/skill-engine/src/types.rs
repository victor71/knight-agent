//! Skill Engine Types
//!
//! Core data types for the skill engine.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Skill engine errors
#[derive(Error, Debug)]
pub enum SkillEngineError {
    #[error("Skill engine not initialized")]
    NotInitialized,
    #[error("Skill not found: {0}")]
    SkillNotFound(String),
    #[error("Skill execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Skill already registered: {0}")]
    AlreadyRegistered(String),
    #[error("Invalid skill definition: {0}")]
    InvalidDefinition(String),
    #[error("Pipeline error: {0}")]
    PipelineError(String),
    #[error("Trigger error: {0}")]
    TriggerError(String),
}

/// Result type for skill engine operations
pub type SkillResult<T> = Result<T, SkillEngineError>;

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub parameters: Vec<SkillParameter>,
    #[serde(default)]
    pub steps: Vec<SkillStep>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub version: String,
}

impl SkillDefinition {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category: None,
            triggers: Vec::new(),
            parameters: Vec::new(),
            steps: Vec::new(),
            enabled: true,
            version: "1.0.0".to_string(),
        }
    }

    pub fn with_category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }

    pub fn with_trigger(mut self, trigger: Trigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    pub fn with_parameter(mut self, param: SkillParameter) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_step(mut self, step: SkillStep) -> Self {
        self.steps.push(step);
        self
    }
}

/// Skill parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
}

impl SkillParameter {
    pub fn new(name: &str, param_type: ParameterType, description: &str) -> Self {
        Self {
            name: name.to_string(),
            param_type,
            description: description.to_string(),
            required: true,
            default_value: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default_value = Some(value);
        self.required = false;
        self
    }
}

/// Parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Object,
    Array,
}

impl Default for ParameterType {
    fn default() -> Self {
        Self::String
    }
}

/// Skill step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub step_type: StepType,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub skill_id: Option<String>,
    #[serde(default)]
    pub parameters: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub condition: Option<String>,
}

impl SkillStep {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            step_type: StepType::Action,
            tool: None,
            skill_id: None,
            parameters: serde_json::Map::new(),
            condition: None,
        }
    }

    pub fn with_tool(mut self, tool: &str) -> Self {
        self.tool = Some(tool.to_string());
        self
    }

    pub fn with_skill(mut self, skill_id: &str) -> Self {
        self.skill_id = Some(skill_id.to_string());
        self.step_type = StepType::Skill;
        self
    }
}

/// Step type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Action,
    Skill,
    Agent,
    Condition,
}

impl Default for StepType {
    fn default() -> Self {
        Self::Action
    }
}

/// Trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub id: String,
    pub trigger_type: TriggerType,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
}

impl Trigger {
    pub fn new(id: &str, trigger_type: TriggerType) -> Self {
        Self {
            id: id.to_string(),
            trigger_type,
            pattern: None,
            event_type: None,
        }
    }

    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.pattern = Some(pattern.to_string());
        self
    }

    pub fn with_event_type(mut self, event_type: &str) -> Self {
        self.event_type = Some(event_type.to_string());
        self
    }
}

/// Trigger type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Keyword,
    Event,
    Timer,
    FileChange,
}

impl Default for TriggerType {
    fn default() -> Self {
        Self::Keyword
    }
}

/// Skill execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub session_id: String,
    #[serde(default)]
    pub variables: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl SkillContext {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            variables: serde_json::Map::new(),
            files: Vec::new(),
            metadata: serde_json::Map::new(),
        }
    }

    pub fn with_variable(mut self, key: &str, value: serde_json::Value) -> Self {
        self.variables.insert(key.to_string(), value);
        self
    }

    pub fn with_file(mut self, file: &str) -> Self {
        self.files.push(file.to_string());
        self
    }
}

/// Skill execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    pub skill_id: String,
    pub success: bool,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub steps_completed: Vec<String>,
    pub execution_time_ms: u64,
}

impl SkillExecutionResult {
    pub fn success(skill_id: &str, output: serde_json::Value, execution_time_ms: u64) -> Self {
        Self {
            skill_id: skill_id.to_string(),
            success: true,
            output: Some(output),
            error: None,
            steps_completed: Vec::new(),
            execution_time_ms,
        }
    }

    pub fn failure(skill_id: &str, error: &str, execution_time_ms: u64) -> Self {
        Self {
            skill_id: skill_id.to_string(),
            success: false,
            output: None,
            error: Some(error.to_string()),
            steps_completed: Vec::new(),
            execution_time_ms,
        }
    }

    pub fn with_steps_completed(mut self, steps: Vec<String>) -> Self {
        self.steps_completed = steps;
        self
    }
}

/// Skill info (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    pub version: String,
}

impl From<&SkillDefinition> for SkillInfo {
    fn from(skill: &SkillDefinition) -> Self {
        Self {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            category: skill.category.clone(),
            enabled: skill.enabled,
            version: skill.version.clone(),
        }
    }
}

/// Pipeline definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<PipelineStep>,
}

impl Pipeline {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            steps: Vec::new(),
        }
    }

    pub fn with_step(mut self, step: PipelineStep) -> Self {
        self.steps.push(step);
        self
    }
}

/// Pipeline step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub skill_id: String,
    #[serde(default)]
    pub parameters: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub condition: Option<String>,
}

impl PipelineStep {
    pub fn new(skill_id: &str) -> Self {
        Self {
            skill_id: skill_id.to_string(),
            parameters: serde_json::Map::new(),
            condition: None,
        }
    }
}

/// Execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub steps: Vec<PlannedStep>,
    #[serde(default)]
    pub confidence: f64,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            confidence: 1.0,
        }
    }
}

/// Planned step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    pub sequence: usize,
    pub skill_id: String,
    pub parameters: serde_json::Map<String, serde_json::Value>,
}

