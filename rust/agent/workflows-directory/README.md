# Workflows Directory Module

Design Reference: `docs/03-module-design/agent/workflows-directory.md`

## Overview

Workflows Directory manages workflow definitions loaded from Markdown files:
- Workflow parsing from Markdown with frontmatter
- Workflow registration and retrieval
- Category and tag-based organization
- Workflow index generation
- Search functionality

## Import

```rust
use workflows_directory::{
    WorkflowDirectoryImpl, WorkflowDirectory,
    WorkflowDefinition, WorkflowMetadata, WorkflowStep,
    WorkflowParameter, WorkflowPrerequisites,
    WorkflowIndexEntry, WorkflowCategory,
    WorkflowParser, parse_index,
};
```

## Core Types

### WorkflowMetadata
Workflow metadata:
- `name`: Workflow name
- `category`: Category name
- `tags`: Tag list
- `description`: Description
- `author`: Author (optional)
- `version`: Version (optional)
- `file_path`: Source file path

### WorkflowStep
Workflow step:
- `step_id`: Step identifier
- `name`: Step name
- `agent`: Agent to use
- `prompt`: Step prompt
- `depends_on`: Dependencies
- `inputs`: Step inputs
- `outputs`: Step outputs
- `parallel_with`: Parallel execution

### WorkflowDefinition
Complete workflow definition:
- `metadata`: Workflow metadata
- `prerequisites`: Prerequisites
- `parameters`: Input parameters
- `steps`: Workflow steps
- `outputs`: Expected outputs
- `notes`: Additional notes

## Usage

### Create Workflow Directory

```rust
let dir = WorkflowDirectoryImpl::new(std::path::PathBuf::from("workflows"));
```

### Initialize Directory

```rust
dir.initialize().await.unwrap();
```

### Register Workflow

```rust
let metadata = WorkflowMetadata::new(
    "feature-dev",
    "software-development",
    "Feature development workflow",
    "workflows/feature-dev.md",
);

let workflow = WorkflowDefinition::new(metadata)
    .with_steps(vec![
        WorkflowStep::new("step1", "Analyze", "architect", "Analyze requirements"),
    ]);

dir.register_workflow(workflow).await.unwrap();
```

### Get Workflow

```rust
let workflow = dir.get_workflow("feature-dev").await.unwrap();
println!("Steps: {}", workflow.steps.len());
```

### List Workflows

```rust
// All workflows
let all = dir.list_workflows().await;

// By category
let testing = dir.list_by_category("testing").await;

// By tag
let api_workflows = dir.list_by_tag("api").await;
```

### Search Workflows

```rust
let results = dir.search("user").await;
```

### Get Categories

```rust
let categories = dir.get_categories().await;
for cat in categories {
    println!("{}: {} workflows", cat.name, cat.workflows.len());
}
```

## Markdown Format

```markdown
---
name: feature-dev
category: software-development
tags: [api, v1]
description: Feature development workflow
author: Dev Team
version: 1.0.0
---

# Feature Development Workflow

## 前置条件

- Git repository must be clean
- All tests passing

## 输入参数

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| feature_name | string | 是 | Feature name |
| description | string | 否 | Feature description |

## 执行步骤

### 步骤 1: Analyze Requirements

使用 **Agent architect** 执行：

```
Analyze the feature requirements
```

输入：
- `req`: 来自 user_input

输出：
- `analysis`: Requirements analysis

### 步骤 2: Implement

使用 **Agent developer** 执行：

在 **步骤 1** 完成后执行：

```
Implement the feature
```

## 输出结果

- `code`: Implementation code
- `tests`: Test cases

## 注意事项

- Ensure code follows style guidelines
- Update documentation as needed
```

## Error Handling

```rust
match dir.register_workflow(workflow).await {
    Ok(_) => println!("Registered!"),
    Err(WorkflowDirectoryError::RegistrationFailed(msg)) => {
        println!("Already exists: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```
