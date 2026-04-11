//! Workflows Directory Types
//!
//! Core data types for the workflows directory module.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Simple workflow step for backwards compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkflowStep {
    pub step_id: String,
    pub action: String,
    pub parameters: serde_json::Value,
}

/// Simple workflow struct for backwards compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<SimpleWorkflowStep>,
}

/// Workflow directory trait
#[allow(async_fn_in_trait)]
pub trait WorkflowDirectory: Send + Sync {
    fn new() -> Result<Self, WorkflowDirectoryError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn register_workflow(&self, workflow: Workflow) -> WorkflowDirectoryResult<()>;
    async fn get_workflow(&self, name: &str) -> WorkflowDirectoryResult<Workflow>;
    async fn list_workflows(&self) -> WorkflowDirectoryResult<Vec<Workflow>>;
}

/// Workflows directory errors
#[derive(Error, Debug)]
pub enum WorkflowDirectoryError {
    #[error("Workflow directory not initialized")]
    NotInitialized,
    #[error("Workflow not found: {0}")]
    NotFound(String),
    #[error("Workflow parsing failed: {0}")]
    ParseError(String),
    #[error("Workflow registration failed: {0}")]
    RegistrationFailed(String),
    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
}

/// Result type for workflow directory operations
pub type WorkflowDirectoryResult<T> = Result<T, WorkflowDirectoryError>;

/// Workflow metadata from frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub file_path: String,
}

impl WorkflowMetadata {
    pub fn new(name: &str, category: &str, description: &str, file_path: &str) -> Self {
        Self {
            name: name.to_string(),
            category: category.to_string(),
            tags: Vec::new(),
            description: description.to_string(),
            author: None,
            version: None,
            file_path: file_path.to_string(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }
}

/// Workflow step input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInput {
    pub name: String,
    pub source: String,
}

impl StepInput {
    pub fn new(name: &str, source: &str) -> Self {
        Self {
            name: name.to_string(),
            source: source.to_string(),
        }
    }
}

/// Workflow step output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub name: String,
    pub description: String,
}

impl StepOutput {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }
}

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: String,
    pub name: String,
    pub agent: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<StepInput>,
    #[serde(default)]
    pub outputs: Vec<StepOutput>,
    #[serde(default)]
    pub parallel_with: Vec<String>,
}

impl WorkflowStep {
    pub fn new(step_id: &str, name: &str, agent: &str, prompt: &str) -> Self {
        Self {
            step_id: step_id.to_string(),
            name: name.to_string(),
            agent: agent.to_string(),
            prompt: prompt.to_string(),
            depends_on: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            parallel_with: Vec::new(),
        }
    }

    pub fn with_depends_on(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn with_inputs(mut self, inputs: Vec<StepInput>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_outputs(mut self, outputs: Vec<StepOutput>) -> Self {
        self.outputs = outputs;
        self
    }

    pub fn with_parallel_with(mut self, parallel: Vec<String>) -> Self {
        self.parallel_with = parallel;
        self
    }
}

/// Workflow prerequisites
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowPrerequisites {
    #[serde(default)]
    pub items: Vec<String>,
}

impl WorkflowPrerequisites {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }
}

/// Workflow parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    #[serde(default)]
    pub default: Option<String>,
}

impl WorkflowParameter {
    pub fn new(name: &str, param_type: &str, required: bool, description: &str) -> Self {
        Self {
            name: name.to_string(),
            param_type: param_type.to_string(),
            required,
            description: description.to_string(),
            default: None,
        }
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }
}

/// Workflow definition (fully parsed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub metadata: WorkflowMetadata,
    #[serde(default)]
    pub prerequisites: WorkflowPrerequisites,
    #[serde(default)]
    pub parameters: Vec<WorkflowParameter>,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl WorkflowDefinition {
    pub fn new(metadata: WorkflowMetadata) -> Self {
        Self {
            metadata,
            prerequisites: WorkflowPrerequisites::default(),
            parameters: Vec::new(),
            steps: Vec::new(),
            outputs: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_prerequisites(mut self, prerequisites: WorkflowPrerequisites) -> Self {
        self.prerequisites = prerequisites;
        self
    }

    pub fn with_parameters(mut self, parameters: Vec<WorkflowParameter>) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_steps(mut self, steps: Vec<WorkflowStep>) -> Self {
        self.steps = steps;
        self
    }

    pub fn with_outputs(mut self, outputs: Vec<String>) -> Self {
        self.outputs = outputs;
        self
    }

    pub fn with_notes(mut self, notes: Vec<String>) -> Self {
        self.notes = notes;
        self
    }
}

/// Workflow category info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCategory {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub workflows: Vec<String>,
}

impl WorkflowCategory {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            workflows: Vec::new(),
        }
    }

    pub fn with_workflows(mut self, workflows: Vec<String>) -> Self {
        self.workflows = workflows;
        self
    }
}

/// Workflow index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIndexEntry {
    pub name: String,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub file_path: String,
}

impl WorkflowIndexEntry {
    pub fn from_definition(def: &WorkflowDefinition) -> Self {
        Self {
            name: def.metadata.name.clone(),
            category: def.metadata.category.clone(),
            description: def.metadata.description.clone(),
            tags: def.metadata.tags.clone(),
            file_path: def.metadata.file_path.clone(),
        }
    }
}

/// Workflow execution info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub metadata: WorkflowMetadata,
    pub step_count: usize,
    pub estimated_duration: Option<String>,
}

impl WorkflowInfo {
    pub fn from_definition(def: &WorkflowDefinition) -> Self {
        Self {
            metadata: def.metadata.clone(),
            step_count: def.steps.len(),
            estimated_duration: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_metadata_new() {
        let meta = WorkflowMetadata::new(
            "feature-dev",
            "software-development",
            "Feature development workflow",
            "workflows/feature-dev.md",
        );
        assert_eq!(meta.name, "feature-dev");
        assert_eq!(meta.category, "software-development");
    }

    #[test]
    fn test_workflow_step_new() {
        let step = WorkflowStep::new("step1", "Analyze", "architect", "Analyze requirements");
        assert_eq!(step.step_id, "step1");
        assert_eq!(step.agent, "architect");
    }

    #[test]
    fn test_workflow_definition_new() {
        let meta = WorkflowMetadata::new("test", "test", "Test workflow", "test.md");
        let def = WorkflowDefinition::new(meta)
            .with_steps(vec![WorkflowStep::new(
                "step1",
                "Step 1",
                "agent",
                "Do something",
            )])
            .with_parameters(vec![WorkflowParameter::new(
                "param1",
                "string",
                true,
                "A parameter",
            )]);

        assert_eq!(def.steps.len(), 1);
        assert_eq!(def.parameters.len(), 1);
    }

    #[test]
    fn test_workflow_index_entry() {
        let meta = WorkflowMetadata::new("test", "cat", "Desc", "test.md")
            .with_tags(vec!["tag1".to_string()]);
        let def = WorkflowDefinition::new(meta);
        let entry = WorkflowIndexEntry::from_definition(&def);
        assert_eq!(entry.tags, vec!["tag1"]);
    }
}
