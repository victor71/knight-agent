//! Command Module Tests
//!
//! Unit tests for the command module.

use command::{
    ArgBinder, CommandArg, CommandDefinition, CommandExecutionContext, CommandManagerImpl,
    CommandMetadata, CommandParser, CommandType, CommandUsage, VariableResolver, WorkflowConfig,
};

// Parser tests

#[tokio::test]
async fn test_parse_simple_command() {
    let content = r#"
---
name: review
description: Execute code review
---

# Command: review

## Usage

```
/review [path]
```

## Args

- `path` (optional): File or directory to review
"#;

    let path = std::path::Path::new("review.md");
    let result = CommandParser::parse_content(content, path).unwrap();

    assert_eq!(result.metadata.name, "review");
    assert_eq!(result.metadata.command_type, CommandType::Simple);
    assert_eq!(result.args.len(), 1);
}

#[tokio::test]
async fn test_parse_workflow_command() {
    let content = r#"
---
name: workflow
description: Execute a workflow
command_type: workflow
---

# Command: workflow

## Usage

```
/workflow <name> [args...]
```
"#;

    let path = std::path::Path::new("workflow.md");
    let result = CommandParser::parse_content(content, path).unwrap();

    assert_eq!(result.metadata.name, "workflow");
    assert_eq!(result.metadata.command_type, CommandType::Workflow);
}

// Manager tests

#[tokio::test]
async fn test_register_command() {
    let cm = CommandManagerImpl::new();

    let cmd =
        CommandDefinition::new("test", "Test command", "/test <arg>", "test.md").with_args(vec![
            CommandArg::new("arg", "An argument").with_required(true),
        ]);

    let result = cm.register_command(cmd).await;
    assert!(result.is_ok());
    assert!(cm.has_command("test").await);
}

#[tokio::test]
async fn test_register_duplicate_command() {
    let cm = CommandManagerImpl::new();

    let cmd1 = CommandDefinition::new("test", "Test 1", "/test", "test.md");
    let cmd2 = CommandDefinition::new("test", "Test 2", "/test", "test.md");

    cm.register_command(cmd1).await.unwrap();
    let result = cm.register_command(cmd2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unregister_command() {
    let cm = CommandManagerImpl::new();

    let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
    cm.register_command(cmd).await.unwrap();

    let result = cm.unregister_command("test").await;
    assert!(result.is_ok());
    assert!(!cm.has_command("test").await);
}

#[tokio::test]
async fn test_list_commands() {
    let cm = CommandManagerImpl::new();

    let cmd1 = CommandDefinition::new("test1", "Test 1", "/test1", "test1.md");
    let cmd2 = CommandDefinition::new("test2", "Test 2", "/test2", "test2.md");

    cm.register_command(cmd1).await.unwrap();
    cm.register_command(cmd2).await.unwrap();

    let commands = cm.list_commands().await;
    assert_eq!(commands.len(), 2);
}

#[tokio::test]
async fn test_execute_command() {
    let cm = CommandManagerImpl::new();

    let cmd = CommandDefinition::new("hello", "Say hello", "/hello <name>", "hello.md")
        .with_args(vec![CommandArg::new("name", "Your name")]);

    cm.register_command(cmd).await.unwrap();

    let result = cm
        .execute_command("hello", "/hello Alice", None)
        .await
        .unwrap();
    assert!(result.success);
    assert!(result.output.contains("Alice"));
}

#[tokio::test]
async fn test_enable_disable_command() {
    let cm = CommandManagerImpl::new();

    let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
    cm.register_command(cmd).await.unwrap();

    cm.disable_command("test").await.unwrap();
    assert_eq!(cm.enabled_command_count().await, 0);

    cm.enable_command("test").await.unwrap();
    assert_eq!(cm.enabled_command_count().await, 1);
}

#[tokio::test]
async fn test_command_count() {
    let cm = CommandManagerImpl::new();
    assert_eq!(cm.command_count().await, 0);

    let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
    cm.register_command(cmd).await.unwrap();

    assert_eq!(cm.command_count().await, 1);
}

// Argument binding tests

#[test]
fn test_arg_binder_positional() {
    let cmd = CommandDefinition::new("test", "Test", "/test <arg>", "test.md").with_args(vec![
        CommandArg::new("arg", "An argument").with_required(true),
    ]);

    let bound = ArgBinder::bind_args(&cmd, &["value".to_string()]).unwrap();
    assert_eq!(bound.get("arg").unwrap().as_str().unwrap(), "value");
}

#[test]
fn test_arg_binder_named() {
    let cmd = CommandDefinition::new("test", "Test", "/test --arg <val>", "test.md")
        .with_args(vec![CommandArg::new("arg", "An argument")]);

    let bound = ArgBinder::bind_args(&cmd, &["--arg".to_string(), "value".to_string()]).unwrap();
    assert_eq!(bound.get("arg").unwrap().as_str().unwrap(), "value");
}

// Variable resolver tests

#[test]
fn test_variable_resolver() {
    let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
    let mut parsed_args = serde_json::Map::new();
    parsed_args.insert(
        "name".to_string(),
        serde_json::Value::String("Alice".to_string()),
    );

    let context = CommandExecutionContext::new(cmd, parsed_args).with_user_input("test input");

    let resolved = VariableResolver::resolve_string("Hello {{ name }}!", &context);
    assert_eq!(resolved, "Hello Alice!");
}

#[test]
fn test_variable_resolver_with_filter() {
    let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
    let mut parsed_args = serde_json::Map::new();
    parsed_args.insert(
        "name".to_string(),
        serde_json::Value::String("alice".to_string()),
    );

    let context = CommandExecutionContext::new(cmd, parsed_args);

    let resolved = VariableResolver::resolve_string("Hello {{ name | upper }}!", &context);
    assert_eq!(resolved, "Hello ALICE!");
}
