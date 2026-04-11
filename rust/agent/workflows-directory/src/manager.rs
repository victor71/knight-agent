//! Workflow Directory Manager
//!
//! Manages workflow loading, registration, and retrieval.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::parser::WorkflowParser;
use crate::types::*;

/// Workflow directory manager implementation
#[derive(Clone)]
pub struct WorkflowDirectoryImpl {
    /// Base directory for workflows
    base_path: PathBuf,
    /// Registered workflows by name
    workflows: Arc<RwLock<std::collections::HashMap<String, WorkflowDefinition>>>,
    /// Workflow index entries
    index: Arc<RwLock<Vec<WorkflowIndexEntry>>>,
    /// Categories
    categories: Arc<RwLock<std::collections::HashMap<String, WorkflowCategory>>>,
    /// Initialization state
    initialized: Arc<RwLock<bool>>,
}

impl WorkflowDirectoryImpl {
    /// Create a new workflow directory manager
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            workflows: Arc::new(RwLock::new(std::collections::HashMap::new())),
            index: Arc::new(RwLock::new(Vec::new())),
            categories: Arc::new(RwLock::new(std::collections::HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Create with default path
    pub fn with_default_path() -> Self {
        Self::new(PathBuf::from("workflows"))
    }

    /// Get the base path
    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Initialize by scanning the workflows directory
    pub async fn initialize(&self) -> WorkflowDirectoryResult<()> {
        if *self.initialized.read().await {
            return Ok(());
        }

        let base_path = &self.base_path;

        // Check if directory exists
        if !base_path.exists() {
            tracing::warn!("Workflows directory does not exist: {:?}", base_path);
            *self.initialized.write().await = true;
            return Ok(());
        }

        // Scan for workflow files
        let mut entries = tokio::fs::read_dir(base_path).await.map_err(|e| {
            WorkflowDirectoryError::InvalidDefinition(format!(
                "Failed to read workflows directory: {}",
                e
            ))
        })?;

        let mut workflows_to_register = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            WorkflowDirectoryError::InvalidDefinition(format!(
                "Failed to read directory entry: {}",
                e
            ))
        })? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                workflows_to_register.push(path);
            }
        }

        // Parse and register each workflow
        for workflow_path in workflows_to_register {
            match WorkflowParser::parse_file(&workflow_path).await {
                Ok(workflow) => {
                    let name = workflow.metadata.name.clone();
                    self.workflows.write().await.insert(name.clone(), workflow);

                    // Update index
                    let entry = WorkflowIndexEntry::from_definition(
                        self.workflows.read().await.get(&name).unwrap(),
                    );
                    self.index.write().await.push(entry);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse workflow {:?}: {}", workflow_path, e);
                }
            }
        }

        // Build category index
        self.rebuild_categories().await?;

        *self.initialized.write().await = true;
        tracing::info!(
            "Workflow directory initialized with {} workflows",
            self.workflows.read().await.len()
        );

        Ok(())
    }

    /// Rebuild categories from loaded workflows
    async fn rebuild_categories(&self) -> WorkflowDirectoryResult<()> {
        let mut categories: std::collections::HashMap<String, WorkflowCategory> =
            std::collections::HashMap::new();

        for workflow in self.workflows.read().await.values() {
            let category_name = workflow.metadata.category.clone();
            let category = categories.entry(category_name.clone()).or_insert_with(|| {
                WorkflowCategory::new(&category_name, &format!("{} workflows", category_name))
            });
            category.workflows.push(workflow.metadata.name.clone());
        }

        *self.categories.write().await = categories;
        Ok(())
    }

    /// Check if initialized
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    /// Register a workflow
    pub async fn register_workflow(
        &self,
        workflow: WorkflowDefinition,
    ) -> WorkflowDirectoryResult<()> {
        let name = workflow.metadata.name.clone();

        // Check for duplicate
        if self.workflows.read().await.contains_key(&name) {
            return Err(WorkflowDirectoryError::RegistrationFailed(format!(
                "Workflow '{}' already registered",
                name
            )));
        }

        self.workflows.write().await.insert(name.clone(), workflow);

        // Update index
        let entry =
            WorkflowIndexEntry::from_definition(self.workflows.read().await.get(&name).unwrap());
        self.index.write().await.push(entry);

        // Rebuild categories
        self.rebuild_categories().await?;

        Ok(())
    }

    /// Get a workflow by name
    pub async fn get_workflow(&self, name: &str) -> WorkflowDirectoryResult<WorkflowDefinition> {
        self.workflows
            .read()
            .await
            .get(name)
            .cloned()
            .ok_or_else(|| WorkflowDirectoryError::NotFound(name.to_string()))
    }

    /// List all workflows
    pub async fn list_workflows(&self) -> Vec<WorkflowDefinition> {
        self.workflows.read().await.values().cloned().collect()
    }

    /// List workflows by category
    pub async fn list_by_category(&self, category: &str) -> Vec<WorkflowDefinition> {
        self.workflows
            .read()
            .await
            .values()
            .filter(|w| w.metadata.category == category)
            .cloned()
            .collect()
    }

    /// List workflows by tag
    pub async fn list_by_tag(&self, tag: &str) -> Vec<WorkflowDefinition> {
        self.workflows
            .read()
            .await
            .values()
            .filter(|w| w.metadata.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Search workflows by name or description
    pub async fn search(&self, query: &str) -> Vec<WorkflowDefinition> {
        let query_lower = query.to_lowercase();
        self.workflows
            .read()
            .await
            .values()
            .filter(|w| {
                w.metadata.name.to_lowercase().contains(&query_lower)
                    || w.metadata.description.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Get workflow index
    pub async fn get_index(&self) -> Vec<WorkflowIndexEntry> {
        self.index.read().await.clone()
    }

    /// Get all categories
    pub async fn get_categories(&self) -> Vec<WorkflowCategory> {
        self.categories.read().await.values().cloned().collect()
    }

    /// Get workflow count
    pub async fn workflow_count(&self) -> usize {
        self.workflows.read().await.len()
    }

    /// Unregister a workflow
    pub async fn unregister_workflow(&self, name: &str) -> WorkflowDirectoryResult<()> {
        if self.workflows.write().await.remove(name).is_none() {
            return Err(WorkflowDirectoryError::NotFound(name.to_string()));
        }

        // Remove from index
        self.index.write().await.retain(|e| e.name != name);

        // Rebuild categories
        self.rebuild_categories().await?;

        Ok(())
    }

    /// Reload workflows from disk
    pub async fn reload(&self) -> WorkflowDirectoryResult<()> {
        *self.workflows.write().await = std::collections::HashMap::new();
        *self.index.write().await = Vec::new();
        *self.categories.write().await = std::collections::HashMap::new();
        *self.initialized.write().await = false;

        self.initialize().await
    }
}

impl WorkflowDirectory for WorkflowDirectoryImpl {
    fn new() -> Result<Self, WorkflowDirectoryError> {
        Ok(Self::with_default_path())
    }

    fn name(&self) -> &str {
        "workflows-directory"
    }

    fn is_initialized(&self) -> bool {
        // Use blocking poll since this is not async
        // Note: This is a sync accessor, use is_initialized_async for actual async check
        false
    }

    async fn register_workflow(&self, workflow: Workflow) -> WorkflowDirectoryResult<()> {
        let def = WorkflowDefinition {
            metadata: WorkflowMetadata::new(
                &workflow.name,
                "custom",
                &workflow.description,
                "memory",
            ),
            prerequisites: WorkflowPrerequisites::default(),
            parameters: Vec::new(),
            steps: workflow
                .steps
                .iter()
                .map(|s| WorkflowStep::new(&s.step_id, &s.action, "", ""))
                .collect(),
            outputs: Vec::new(),
            notes: Vec::new(),
        };
        self.register_workflow(def).await
    }

    async fn get_workflow(&self, name: &str) -> WorkflowDirectoryResult<Workflow> {
        let def = self.get_workflow(name).await?;
        Ok(Workflow {
            name: def.metadata.name,
            description: def.metadata.description,
            steps: def
                .steps
                .iter()
                .map(|s| SimpleWorkflowStep {
                    step_id: s.step_id.clone(),
                    action: s.name.clone(),
                    parameters: serde_json::json!({}),
                })
                .collect(),
        })
    }

    async fn list_workflows(&self) -> WorkflowDirectoryResult<Vec<Workflow>> {
        Ok(self
            .list_workflows()
            .await
            .into_iter()
            .map(|def| Workflow {
                name: def.metadata.name,
                description: def.metadata.description,
                steps: def
                    .steps
                    .into_iter()
                    .map(|s| SimpleWorkflowStep {
                        step_id: s.step_id,
                        action: s.name,
                        parameters: serde_json::json!({}),
                    })
                    .collect(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_directory_impl_new() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));
        assert_eq!(dir.name(), "workflows-directory");
        assert!(!dir.is_initialized().await);
    }

    #[tokio::test]
    async fn test_register_and_get_workflow() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let metadata =
            WorkflowMetadata::new("test-workflow", "testing", "A test workflow", "test.md");
        let workflow = WorkflowDefinition::new(metadata);

        dir.register_workflow(workflow.clone()).await.unwrap();

        let retrieved = dir.get_workflow("test-workflow").await.unwrap();
        assert_eq!(retrieved.metadata.name, "test-workflow");
    }

    #[tokio::test]
    async fn test_get_nonexistent_workflow() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let result = dir.get_workflow("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_workflows() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let wf1 = WorkflowDefinition::new(WorkflowMetadata::new("wf1", "cat1", "Desc1", "wf1.md"));
        let wf2 = WorkflowDefinition::new(WorkflowMetadata::new("wf2", "cat2", "Desc2", "wf2.md"));

        dir.register_workflow(wf1).await.unwrap();
        dir.register_workflow(wf2).await.unwrap();

        let workflows = dir.list_workflows().await;
        assert_eq!(workflows.len(), 2);
    }

    #[tokio::test]
    async fn test_list_by_category() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let wf1 =
            WorkflowDefinition::new(WorkflowMetadata::new("wf1", "testing", "Desc1", "wf1.md"));
        let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
            "wf2",
            "production",
            "Desc2",
            "wf2.md",
        ));

        dir.register_workflow(wf1).await.unwrap();
        dir.register_workflow(wf2).await.unwrap();

        let testing = dir.list_by_category("testing").await;
        assert_eq!(testing.len(), 1);
        assert_eq!(testing[0].metadata.name, "wf1");
    }

    #[tokio::test]
    async fn test_list_by_tag() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let mut wf1 =
            WorkflowDefinition::new(WorkflowMetadata::new("wf1", "testing", "Desc1", "wf1.md"));
        wf1.metadata.tags = vec!["api".to_string(), "v1".to_string()];

        dir.register_workflow(wf1).await.unwrap();

        let tagged = dir.list_by_tag("api").await;
        assert_eq!(tagged.len(), 1);
    }

    #[tokio::test]
    async fn test_search() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let wf1 = WorkflowDefinition::new(WorkflowMetadata::new(
            "user-service",
            "backend",
            "User management service",
            "user.md",
        ));
        let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
            "order-service",
            "backend",
            "Order processing",
            "order.md",
        ));

        dir.register_workflow(wf1).await.unwrap();
        dir.register_workflow(wf2).await.unwrap();

        let results = dir.search("user").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "user-service");
    }

    #[tokio::test]
    async fn test_unregister_workflow() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let wf = WorkflowDefinition::new(WorkflowMetadata::new("wf1", "cat1", "Desc1", "wf1.md"));
        dir.register_workflow(wf).await.unwrap();

        assert!(dir.get_workflow("wf1").await.is_ok());

        dir.unregister_workflow("wf1").await.unwrap();
        assert!(dir.get_workflow("wf1").await.is_err());
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let wf1 = WorkflowDefinition::new(WorkflowMetadata::new("wf1", "cat1", "Desc1", "wf1.md"));
        let wf2 = WorkflowDefinition::new(WorkflowMetadata::new("wf1", "cat2", "Desc2", "wf2.md"));

        dir.register_workflow(wf1).await.unwrap();
        let result = dir.register_workflow(wf2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_categories() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let wf1 =
            WorkflowDefinition::new(WorkflowMetadata::new("wf1", "testing", "Desc1", "wf1.md"));
        let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
            "wf2",
            "production",
            "Desc2",
            "wf2.md",
        ));

        dir.register_workflow(wf1).await.unwrap();
        dir.register_workflow(wf2).await.unwrap();

        let categories = dir.get_categories().await;
        assert_eq!(categories.len(), 2);
    }

    #[tokio::test]
    async fn test_workflow_count() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));
        assert_eq!(dir.workflow_count().await, 0);

        let wf = WorkflowDefinition::new(WorkflowMetadata::new("wf1", "cat1", "Desc1", "wf1.md"));
        dir.register_workflow(wf).await.unwrap();

        assert_eq!(dir.workflow_count().await, 1);
    }

    #[tokio::test]
    async fn test_get_index() {
        let dir = WorkflowDirectoryImpl::new(PathBuf::from("workflows"));

        let wf =
            WorkflowDefinition::new(WorkflowMetadata::new("wf1", "testing", "Desc1", "wf1.md"));
        dir.register_workflow(wf).await.unwrap();

        let index = dir.get_index().await;
        assert_eq!(index.len(), 1);
        assert_eq!(index[0].name, "wf1");
    }
}
