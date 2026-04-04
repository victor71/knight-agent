//! Workflows Directory Tests
//!
//! Unit tests for the workflows directory module.

use workflows_directory::{
    WorkflowDirectoryImpl, WorkflowDefinition, WorkflowMetadata,
    WorkflowStep, WorkflowPrerequisites, WorkflowParameter,
    WorkflowParser, parse_index, WorkflowIndexEntry, WorkflowCategory,
    WorkflowDirectory,
};

#[tokio::test]
async fn test_workflow_directory_impl_new() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));
    assert_eq!(dir.name(), "workflows-directory");
}

#[tokio::test]
async fn test_register_workflow() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let metadata = WorkflowMetadata::new(
        "test-workflow",
        "testing",
        "A test workflow",
        "test.md",
    );
    let workflow = WorkflowDefinition::new(metadata);

    let result = dir.register_workflow(workflow).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_workflow() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let metadata = WorkflowMetadata::new(
        "test-workflow",
        "testing",
        "A test workflow",
        "test.md",
    );
    let workflow = WorkflowDefinition::new(metadata);

    dir.register_workflow(workflow).await.unwrap();

    let retrieved = dir.get_workflow("test-workflow").await.unwrap();
    assert_eq!(retrieved.metadata.name, "test-workflow");
    assert_eq!(retrieved.metadata.category, "testing");
}

#[tokio::test]
async fn test_get_nonexistent_workflow() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let result = dir.get_workflow("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_workflows() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let wf1 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "cat1", "Desc1", "wf1.md",
    ));
    let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf2", "cat2", "Desc2", "wf2.md",
    ));

    dir.register_workflow(wf1).await.unwrap();
    dir.register_workflow(wf2).await.unwrap();

    let workflows = dir.list_workflows().await;
    assert_eq!(workflows.len(), 2);
}

#[tokio::test]
async fn test_list_by_category() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let wf1 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "testing", "Desc1", "wf1.md",
    ));
    let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf2", "production", "Desc2", "wf2.md",
    ));

    dir.register_workflow(wf1).await.unwrap();
    dir.register_workflow(wf2).await.unwrap();

    let testing = dir.list_by_category("testing").await;
    assert_eq!(testing.len(), 1);
    assert_eq!(testing[0].metadata.name, "wf1");

    let production = dir.list_by_category("production").await;
    assert_eq!(production.len(), 1);
    assert_eq!(production[0].metadata.name, "wf2");
}

#[tokio::test]
async fn test_list_by_tag() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let mut wf1 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "testing", "Desc1", "wf1.md",
    ));
    wf1.metadata.tags = vec!["api".to_string(), "v1".to_string()];

    let mut wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf2", "testing", "Desc2", "wf2.md",
    ));
    wf2.metadata.tags = vec!["web".to_string()];

    dir.register_workflow(wf1).await.unwrap();
    dir.register_workflow(wf2).await.unwrap();

    let tagged = dir.list_by_tag("api").await;
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].metadata.name, "wf1");
}

#[tokio::test]
async fn test_search() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let wf1 = WorkflowDefinition::new(WorkflowMetadata::new(
        "user-service", "backend", "User management service", "user.md",
    ));
    let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
        "order-service", "backend", "Order processing", "order.md",
    ));

    dir.register_workflow(wf1).await.unwrap();
    dir.register_workflow(wf2).await.unwrap();

    let results = dir.search("user").await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.name, "user-service");

    let results2 = dir.search("service").await;
    assert_eq!(results2.len(), 2);
}

#[tokio::test]
async fn test_unregister_workflow() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let wf = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "cat1", "Desc1", "wf1.md",
    ));
    dir.register_workflow(wf).await.unwrap();

    assert!(dir.get_workflow("wf1").await.is_ok());

    let result = dir.unregister_workflow("wf1").await;
    assert!(result.is_ok());

    assert!(dir.get_workflow("wf1").await.is_err());
}

#[tokio::test]
async fn test_duplicate_registration() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let wf1 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "cat1", "Desc1", "wf1.md",
    ));
    let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "cat2", "Desc2", "wf2.md",
    ));

    dir.register_workflow(wf1).await.unwrap();
    let result = dir.register_workflow(wf2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_categories() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let wf1 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "testing", "Desc1", "wf1.md",
    ));
    let wf2 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf2", "production", "Desc2", "wf2.md",
    ));
    let wf3 = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf3", "testing", "Desc3", "wf3.md",
    ));

    dir.register_workflow(wf1).await.unwrap();
    dir.register_workflow(wf2).await.unwrap();
    dir.register_workflow(wf3).await.unwrap();

    let categories = dir.get_categories().await;
    assert_eq!(categories.len(), 2);

    // Find testing category
    let testing_cat = categories.iter().find(|c| c.name == "testing").unwrap();
    assert_eq!(testing_cat.workflows.len(), 2);
}

#[tokio::test]
async fn test_workflow_count() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));
    assert_eq!(dir.workflow_count().await, 0);

    let wf = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "cat1", "Desc1", "wf1.md",
    ));
    dir.register_workflow(wf).await.unwrap();

    assert_eq!(dir.workflow_count().await, 1);
}

#[tokio::test]
async fn test_get_index() {
    let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));

    let wf = WorkflowDefinition::new(WorkflowMetadata::new(
        "wf1", "testing", "Desc1", "wf1.md",
    ));
    dir.register_workflow(wf).await.unwrap();

    let index = dir.get_index().await;
    assert_eq!(index.len(), 1);
    assert_eq!(index[0].name, "wf1");
    assert_eq!(index[0].category, "testing");
}

// Parser tests

#[test]
fn test_parse_simple_workflow() {
    let content = r#"
---
name: test-workflow
category: testing
description: A simple test workflow
---

# Test Workflow

## 前置条件

- Prereq 1
- Prereq 2

## 输入参数

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| input1 | string | 是 | Input description |

## 执行步骤

### 步骤 1: First Step

使用 **Agent developer** 执行：

```
Do something
```

输入：
- `param1`: 来自 {{ input1 }}

输出：
- `result1`: The result

## 输出结果

- Output 1
- Output 2
"#;

    let path = std::path::Path::new("workflows/test.md");
    let result = WorkflowParser::parse_content(content, path).unwrap();

    assert_eq!(result.metadata.name, "test-workflow");
    assert_eq!(result.metadata.category, "testing");
    assert_eq!(result.metadata.description, "A simple test workflow");
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.steps[0].step_id, "step1");
}

#[test]
fn test_parse_workflow_with_dependencies() {
    let content = r#"
---
name: dependent-workflow
category: testing
description: Workflow with step dependencies
---

# Dependent Workflow

## 执行步骤

### 步骤 1: First Step

使用 **Agent developer** 执行：

```
Do first thing
```

### 步骤 2: Second Step

使用 **Agent reviewer** 执行：

在 **步骤 1** 完成后执行：

```
Do second thing
```
"#;

    let path = std::path::Path::new("workflows/dependent.md");
    let result = WorkflowParser::parse_content(content, path).unwrap();

    assert_eq!(result.steps.len(), 2);
    assert_eq!(result.steps[1].step_id, "step2");
}

#[test]
fn test_parse_workflow_prerequisites() {
    let content = r#"
---
name: prereq-workflow
category: testing
description: Workflow with prerequisites
---

# Prereq Workflow

## 前置条件

- Docker must be installed
- Node.js 18+ required
- Git repository must be clean

## 执行步骤

### 步骤 1: Build

使用 **Agent developer** 执行：

```
Build the project
```
"#;

    let path = std::path::Path::new("workflows/prereq.md");
    let result = WorkflowParser::parse_content(content, path).unwrap();

    assert_eq!(result.prerequisites.items.len(), 3);
    assert!(result.prerequisites.items.contains(&"Docker must be installed".to_string()));
}

#[test]
fn test_parse_workflow_parameters() {
    let content = r#"
---
name: param-workflow
category: testing
description: Workflow with parameters
---

# Param Workflow

## 输入参数

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| name | string | 是 | User name |
| age | number | 否 | User age (default: 18) |
| active | boolean | 否 | Is active |

## 执行步骤

### 步骤 1: Process

使用 **Agent processor** 执行：

```
Process user data
```
"#;

    let path = std::path::Path::new("workflows/param.md");
    let result = WorkflowParser::parse_content(content, path).unwrap();

    assert_eq!(result.parameters.len(), 3);
    assert_eq!(result.parameters[0].name, "name");
    assert!(result.parameters[0].required);
    assert_eq!(result.parameters[1].name, "age");
    assert!(!result.parameters[1].required);
}

#[test]
fn test_parse_workflow_outputs() {
    let content = r#"
---
name: output-workflow
category: testing
description: Workflow with outputs
---

# Output Workflow

## 输出结果

- `user_id`: The created user ID
- `token`: Authentication token
- `profile_url`: URL to user profile

## 执行步骤

### 步骤 1: Create User

使用 **Agent creator** 执行：

```
Create a new user
```
"#;

    let path = std::path::Path::new("workflows/output.md");
    let result = WorkflowParser::parse_content(content, path).unwrap();

    assert_eq!(result.outputs.len(), 3);
    assert!(result.outputs[0].contains("user_id"));
}

#[test]
fn test_parse_workflow_notes() {
    let content = r#"
---
name: note-workflow
category: testing
description: Workflow with notes
---

# Note Workflow

## 注意事项

- Ensure network connectivity
- Check rate limits before execution
- Log all API calls for auditing

## 执行步骤

### 步骤 1: Execute

使用 **Agent executor** 执行：

```
Execute the task
```
"#;

    let path = std::path::Path::new("workflows/note.md");
    let result = WorkflowParser::parse_content(content, path).unwrap();

    assert_eq!(result.notes.len(), 3);
}

#[test]
fn test_parse_index() {
    let content = r#"
# Workflows

### Software Development
- [feature-dev](software-development/feature-dev.md) - Feature development workflow
- [bug-fix](software-development/bug-fix.md) - Bug fix workflow

### Code Quality
- [code-review](code-quality/code-review.md) - Code review workflow
"#;

    let entries = parse_index(content);
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].category, "Software Development");
    assert_eq!(entries[0].name, "feature-dev");
    assert_eq!(entries[1].category, "Software Development");
    assert_eq!(entries[2].category, "Code Quality");
}

// Type tests

#[test]
fn test_workflow_metadata_builder() {
    let meta = WorkflowMetadata::new(
        "test",
        "testing",
        "Test workflow",
        "test.md",
    )
    .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
    .with_author("Test Author")
    .with_version("1.0.0");

    assert_eq!(meta.name, "test");
    assert_eq!(meta.category, "testing");
    assert_eq!(meta.tags.len(), 2);
    assert_eq!(meta.author, Some("Test Author".to_string()));
    assert_eq!(meta.version, Some("1.0.0".to_string()));
}

#[test]
fn test_workflow_step_builder() {
    let step = WorkflowStep::new("step1", "Test Step", "developer", "Do something")
        .with_depends_on(vec!["step0".to_string()])
        .with_inputs(vec![workflows_directory::StepInput::new("input1", "source1")])
        .with_outputs(vec![workflows_directory::StepOutput::new("output1", "Result description")]);

    assert_eq!(step.step_id, "step1");
    assert_eq!(step.name, "Test Step");
    assert_eq!(step.agent, "developer");
    assert_eq!(step.depends_on.len(), 1);
    assert_eq!(step.inputs.len(), 1);
    assert_eq!(step.outputs.len(), 1);
}

#[test]
fn test_workflow_definition_builder() {
    let meta = WorkflowMetadata::new("test", "cat", "Desc", "test.md");
    let step = WorkflowStep::new("step1", "Step 1", "agent", "Do something");

    let def = WorkflowDefinition::new(meta)
        .with_steps(vec![step.clone()])
        .with_parameters(vec![
            WorkflowParameter::new("param1", "string", true, "A parameter"),
        ])
        .with_prerequisites(WorkflowPrerequisites::new())
        .with_outputs(vec!["output1".to_string()])
        .with_notes(vec!["note1".to_string()]);

    assert_eq!(def.steps.len(), 1);
    assert_eq!(def.parameters.len(), 1);
    assert_eq!(def.outputs.len(), 1);
    assert_eq!(def.notes.len(), 1);
}

#[test]
fn test_workflow_index_entry_from_definition() {
    let mut meta = WorkflowMetadata::new("test", "cat", "Desc", "test.md");
    meta.tags = vec!["tag1".to_string()];
    let def = WorkflowDefinition::new(meta);

    let entry = WorkflowIndexEntry::from_definition(&def);
    assert_eq!(entry.name, "test");
    assert_eq!(entry.category, "cat");
    assert_eq!(entry.tags, vec!["tag1"]);
}

#[test]
fn test_workflow_category_new() {
    let cat = WorkflowCategory::new("testing", "Testing workflows")
        .with_workflows(vec!["wf1".to_string(), "wf2".to_string()]);

    assert_eq!(cat.name, "testing");
    assert_eq!(cat.workflows.len(), 2);
}
