# Command Module

Design Reference: `docs/03-module-design/core/command.md`

## Overview

Command module manages CLI command definitions, parsing, and execution:
- Command definition parsing from Markdown format
- Argument binding and validation
- Variable resolution with builtin functions
- Command registry and management
- Simple and workflow command types

## Import

```rust
use command::{
    CommandManagerImpl, CommandError, CommandResult, CommandType,
    CommandDefinition, CommandArg, CommandMetadata, CommandUsage,
    CommandExecutionContext, CommandExecutionResult, CommandInfo,
    WorkflowConfig, ParsedArgs, BuiltinFunction,
    CommandParser, ArgBinder, VariableResolver, CommandConfig,
};
```

## Core Types

### CommandError
Command errors:
- `NotInitialized`: Command not initialized
- `NotFound(String)`: Command not found
- `ParseError(String)`: Command parsing failed
- `ExecutionFailed(String)`: Command execution failed
- `ArgError(String)`: Argument error
- `WorkflowNotFound(String)`: Workflow not found
- `InvalidDefinition(String)`: Invalid command definition
- `VariableError(String)`: Variable resolution error

### CommandType
Command type enumeration:
- `Simple`: Simple command (direct execution)
- `Workflow`: Workflow command (via Task Manager)

### CommandMetadata
Command metadata:
- `name`: Command name
- `description`: Command description
- `version`: Version (optional)
- `author`: Author (optional)
- `file_path`: Definition file path
- `command_type`: Command type

### CommandArg
Command argument:
- `name`: Argument name
- `description`: Argument description
- `required`: Whether required
- `type_hint`: Type hint (optional)
- `default`: Default value (optional)

## Usage

### Create Command Manager

```rust
let cm = CommandManagerImpl::new();
```

### Register Command

```rust
let cmd = CommandDefinition::new("review", "Code review", "/review [path]", "review.md")
    .with_args(vec![
        CommandArg::new("path", "File to review").with_required(false),
    ]);

cm.register_command(cmd).await.unwrap();
```

### Execute Command

```rust
let result = cm.execute_command("hello", "/hello Alice", None).await.unwrap();
println!("Output: {}", result.output);
```

### List Commands

```rust
let commands = cm.list_commands().await;
for cmd in commands {
    println!("- {}: {}", cmd.name, cmd.description);
}
```

### Enable/Disable Command

```rust
cm.disable_command("test").await.unwrap();
cm.enable_command("test").await.unwrap();
```

## Argument Binding

### Positional Arguments

```rust
let cmd = CommandDefinition::new("test", "Test", "/test <arg>", "test.md")
    .with_args(vec![CommandArg::new("arg", "An argument")]);

let bound = ArgBinder::bind_args(&cmd, &["value".to_string()]).unwrap();
```

### Named Arguments

```rust
let bound = ArgBinder::bind_args(&cmd, &["--arg".to_string(), "value".to_string()]).unwrap();
```

## Variable Resolution

```rust
let context = CommandExecutionContext::new(cmd, parsed_args)
    .with_user_input("test input");

let resolved = VariableResolver::resolve_string("Hello {{ name }}!", &context);
// Output: "Hello Alice!"
```

### Builtin Functions

```rust
// {{ timestamp }} - Current timestamp
// {{ date("%Y-%m-%d") }} - Formatted date
// {{ name | upper }} - Uppercase filter
// {{ name | lower }} - Lowercase filter
// {{ value | default: "foo" }} - Default value
```

## Markdown Format

```markdown
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

## Expected Behavior

When user runs `/review`, analyze code quality...
```

## Error Handling

```rust
match cm.register_command(cmd).await {
    Ok(_) => println!("Registered!"),
    Err(CommandError::InvalidDefinition(msg)) => {
        println!("Invalid: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```
