//! Command Parser
//!
//! Parses Markdown command definitions.

use std::collections::HashMap;
use std::path::Path;

use crate::types::*;

/// Command parser for Markdown definitions
pub struct CommandParser;

impl CommandParser {
    /// Parse a Markdown command definition from a file
    pub async fn parse_file(path: &Path) -> CommandResult<CommandDefinition> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| CommandError::ParseError(format!("Failed to read file: {}", e)))?;

        Self::parse_content(&content, path)
    }

    /// Parse a Markdown command definition from content
    pub fn parse_content(content: &str, path: &Path) -> CommandResult<CommandDefinition> {
        // Extract front matter
        let (front_matter, body) = Self::extract_frontmatter(content)?;

        // Parse front matter
        let metadata = Self::parse_frontmatter(&front_matter, path)?;

        // Extract usage section
        let usage = Self::extract_usage(body)?;

        // Extract args section
        let args = Self::extract_args(body)?;

        // Extract expected behavior
        let expected_behavior = Self::extract_expected_behavior(body)?;

        // Extract workflow config if present
        let workflow_config = Self::extract_workflow_config(body)?;

        Ok(CommandDefinition {
            metadata,
            usage,
            args,
            workflow_config,
        })
    }

    /// Extract YAML front matter from Markdown content
    fn extract_frontmatter(content: &str) -> CommandResult<(String, &str)> {
        let trimmed = content.trim();

        if trimmed.starts_with("---") {
            let end_idx = trimmed[3..]
                .find("---")
                .ok_or_else(|| CommandError::ParseError("Front matter not closed".to_string()))?;

            let front_matter = trimmed[3..end_idx + 3].trim().to_string();
            let body = &trimmed[end_idx + 6..];
            Ok((front_matter, body))
        } else {
            Err(CommandError::ParseError(
                "Missing front matter (---)".to_string(),
            ))
        }
    }

    /// Parse front matter into CommandMetadata
    fn parse_frontmatter(front_matter: &str, path: &Path) -> CommandResult<CommandMetadata> {
        let mut name = None;
        let mut description = None;
        let mut version = None;
        let mut author = None;
        let mut command_type = CommandType::Simple;

        for line in front_matter.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "name" => name = Some(value.to_string()),
                    "description" => description = Some(value.to_string()),
                    "version" => version = Some(value.to_string()),
                    "author" => author = Some(value.to_string()),
                    "command_type" => {
                        command_type = match value {
                            "workflow" => CommandType::Workflow,
                            _ => CommandType::Simple,
                        }
                    }
                    _ => {}
                }
            }
        }

        let name = name.ok_or_else(|| CommandError::ParseError("Missing 'name' in front matter".to_string()))?;
        let description = description.ok_or_else(|| CommandError::ParseError("Missing 'description' in front matter".to_string()))?;

        let file_path = path.to_string_lossy().to_string();

        Ok(CommandMetadata {
            name,
            description,
            version,
            author,
            file_path,
            command_type,
        })
    }

    /// Extract Usage section from body
    fn extract_usage(body: &str) -> CommandResult<CommandUsage> {
        let mut syntax = String::new();
        let examples = Vec::new();

        if let Some(usage_start) = body.find("## Usage") {
            // Skip past "## Usage" to find the next section
            let after_usage = &body[usage_start + 9..];

            // Find the next section (## Something) after "## Usage"
            if let Some(usage_end) = after_usage.find("## ") {
                let usage_content = &after_usage[..usage_end];
                for line in usage_content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("```") {
                        continue;
                    }
                    if trimmed.starts_with('/') {
                        syntax = trimmed.to_string();
                        break;
                    }
                }
            } else {
                // Rest of content is usage section
                for line in after_usage.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("```") {
                        continue;
                    }
                    if trimmed.starts_with('/') {
                        syntax = trimmed.to_string();
                        break;
                    }
                }
            }
        }

        if syntax.is_empty() {
            return Err(CommandError::ParseError("Missing Usage section".to_string()));
        }

        Ok(CommandUsage {
            syntax,
            examples,
            expected_behavior: None,
        })
    }

    /// Extract Args section from body
    fn extract_args(body: &str) -> CommandResult<Vec<CommandArg>> {
        let mut args = Vec::new();

        if let Some(args_start) = body.find("## Args") {
            let after_args = &body[args_start + 8..];
            if let Some(args_end) = after_args.find("## ") {
                let args_content = &after_args[..args_end];
                args = Self::parse_args_block(args_content);
            } else {
                args = Self::parse_args_block(after_args);
            }
        }

        Ok(args)
    }

    /// Parse args from block content
    fn parse_args_block(content: &str) -> Vec<CommandArg> {
        let mut args = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse arg definition like `- name: path` or `- path (optional): description`
            if trimmed.starts_with('-') || trimmed.starts_with('`') {
                let clean = trimmed.trim_start_matches('-').trim_start_matches('`').trim();
                if clean.is_empty() {
                    continue;
                }

                // Parse format: "name: description" or "name (optional): description"
                let (name_part, desc_part) = if let Some((n, d)) = clean.split_once(':') {
                    (n.trim(), d.trim())
                } else {
                    (clean, "")
                };

                // Extract name and optional default
                let (name, default) = if let Some(paren_start) = name_part.find('(') {
                    let name = name_part[..paren_start].trim();
                    let paren_content = &name_part[paren_start..];
                    let default = if paren_content.contains("optional") || paren_content.contains("=") {
                        paren_content
                            .trim_start_matches('(')
                            .trim_end_matches(')')
                            .split('=')
                            .nth(1)
                            .map(|s| s.trim().to_string())
                    } else {
                        None
                    };
                    (name.to_string(), default)
                } else {
                    (name_part.to_string(), None)
                };

                let required = default.is_none();

                if !name.is_empty() {
                    args.push(CommandArg {
                        name,
                        description: desc_part.to_string(),
                        required,
                        type_hint: None,
                        default,
                    });
                }
            }
        }

        args
    }

    /// Extract Expected Behavior section
    fn extract_expected_behavior(body: &str) -> CommandResult<Option<String>> {
        let behavior = if let Some(eb_start) = body.find("## Expected Behavior") {
            let eb_section = &body[eb_start..];
            let eb_end = eb_section[19..]
                .find("## ")
                .unwrap_or(eb_section.len() - 19);

            let eb_content = &eb_section[19..eb_end + 19];
            Some(eb_content.trim().to_string())
        } else {
            None
        };

        Ok(behavior)
    }

    /// Extract Workflow Config section
    fn extract_workflow_config(body: &str) -> CommandResult<Option<WorkflowConfig>> {
        if !body.contains("## Workflow") && !body.contains("workflow") {
            return Ok(None);
        }

        let mut config = WorkflowConfig::default();

        // Look for workflow-specific fields
        if body.contains("dynamic_agent_creation") || body.contains("dynamic agent creation") {
            config.dynamic_agent_creation = true;
        }

        if body.contains("parallel_execution") || body.contains("parallel execution") {
            config.parallel_execution = true;
        }

        // Try to find workflow definition path
        for line in body.lines() {
            if line.contains("workflow_definition_path") || line.contains("workflow definition path") {
                // Extract path if present
                if let Some(path_start) = line.find('/') {
                    let path = line[path_start..].trim_matches(|c| c == '`' || c == ' ' || c == '"');
                    if !path.is_empty() && path.starts_with("workflows/") {
                        config.workflow_definition_path = Some(path.to_string());
                    }
                }
            }
        }

        if config.workflow_definition_path.is_some()
            || config.dynamic_agent_creation
            || config.parallel_execution
        {
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }
}

/// Argument binder
pub struct ArgBinder;

impl ArgBinder {
    /// Bind command-line arguments to a command definition
    pub fn bind_args(
        definition: &CommandDefinition,
        input_args: &[String],
    ) -> CommandResult<ParsedArgs> {
        let mut bound: ParsedArgs = serde_json::Map::new();
        let mut used_args: HashMap<usize, bool> = HashMap::new();

        // First pass: handle named arguments (--name value)
        for (i, arg) in input_args.iter().enumerate() {
            if arg.starts_with("--") {
                let name = arg[2..].splitn(2, '=').next().unwrap_or(&arg[2..]);
                let value = if arg.contains('=') {
                    arg.splitn(2, '=').nth(1).unwrap_or("")
                } else if i + 1 < input_args.len() && !input_args[i + 1].starts_with('-') {
                    used_args.insert(i + 1, true);
                    &input_args[i + 1]
                } else {
                    ""
                };

                bound.insert(name.to_string(), serde_json::Value::String(value.to_string()));
            }
        }

        // Second pass: handle positional arguments
        let positional_args: Vec<&String> = input_args
            .iter()
            .enumerate()
            .filter(|(i, _)| !used_args.contains_key(i))
            .map(|(_, a)| a)
            .collect();

        for (i, arg_def) in definition.args.iter().enumerate() {
            // Skip if already bound as named argument
            if bound.contains_key(&arg_def.name) {
                continue;
            }

            if arg_def.name.starts_with("arg_") || arg_def.name == "args" {
                // Skip internal argument names
                continue;
            }

            if positional_args.len() > i {
                bound.insert(
                    arg_def.name.clone(),
                    serde_json::Value::String(positional_args[i].clone()),
                );
            } else if arg_def.required {
                return Err(CommandError::ArgError(format!(
                    "Missing required argument: {}",
                    arg_def.name
                )));
            } else if let Some(ref default) = arg_def.default {
                bound.insert(
                    arg_def.name.clone(),
                    serde_json::Value::String(default.clone()),
                );
            }
        }

        Ok(bound)
    }
}

/// Variable resolver
pub struct VariableResolver;

impl VariableResolver {
    /// Resolve variables in a string using the execution context
    pub fn resolve_string(input: &str, context: &CommandExecutionContext) -> String {
        let mut result = input.to_string();

        // Replace {{ variable }} patterns
        while let Some(start) = result.find("{{") {
            if let Some(end) = result[start..].find("}}") {
                let var_expr = &result[start + 2..start + end];
                let value = Self::resolve_variable(var_expr.trim(), context);
                result = format!("{}{}{}", &result[..start], value, &result[start + end + 2..]);
            } else {
                break;
            }
        }

        result
    }

    /// Resolve a single variable expression
    fn resolve_variable(expr: &str, context: &CommandExecutionContext) -> String {
        // Check for filters (e.g., "name | upper")
        let (var_name, filters) = if let Some(pipe_idx) = expr.find('|') {
            (expr[..pipe_idx].trim(), Some(expr[pipe_idx + 1..].trim()))
        } else {
            (expr.trim(), None)
        };

        // Get base value
        let value = if var_name == "input" || var_name == "user_input" {
            context.user_input.clone()
        } else if var_name == "session_id" {
            context.session_id.clone().unwrap_or_default()
        } else if let Some(val) = context.parsed_args.get(var_name) {
            val.as_str().unwrap_or(var_name).to_string()
        } else {
            var_name.to_string()
        };

        // Apply filters
        if let Some(filters_str) = filters {
            let mut final_value = value;
            for filter in filters_str.split(',') {
                let filter = filter.trim();
                if let Some((filter_name, filter_arg)) = filter.split_once('(') {
                    let arg = filter_arg.trim_end_matches(')');
                    if let Some(func) = BuiltinFunction::parse(filter_name) {
                        final_value = func.apply(&final_value, Some(arg));
                    }
                } else if let Some(func) = BuiltinFunction::parse(filter) {
                    final_value = func.apply(&final_value, None);
                }
            }
            final_value
        } else {
            value
        }
    }

    /// Resolve variables in a JSON value
    pub fn resolve_value(value: &serde_json::Value, context: &CommandExecutionContext) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                serde_json::Value::String(Self::resolve_string(s, context))
            }
            serde_json::Value::Object(map) => {
                let resolved: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::resolve_value(v, context)))
                    .collect();
                serde_json::Value::Object(resolved)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(
                    arr.iter().map(|v| Self::resolve_value(v, context)).collect()
                )
            }
            _ => value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
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

        let path = Path::new("review.md");
        let result = CommandParser::parse_content(content, path).unwrap();

        assert_eq!(result.metadata.name, "review");
        assert_eq!(result.metadata.command_type, CommandType::Simple);
        assert_eq!(result.args.len(), 1);
    }

    #[test]
    fn test_parse_workflow_command() {
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

        let path = Path::new("workflow.md");
        let result = CommandParser::parse_content(content, path).unwrap();

        assert_eq!(result.metadata.name, "workflow");
        assert_eq!(result.metadata.command_type, CommandType::Workflow);
    }

    #[test]
    fn test_arg_binder_positional() {
        let cmd = CommandDefinition::new("test", "Test", "/test <arg>", "test.md")
            .with_args(vec![
                CommandArg::new("arg", "An argument").with_required(true),
            ]);

        let bound = ArgBinder::bind_args(&cmd, &["value".to_string()]).unwrap();
        assert_eq!(bound.get("arg").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_arg_binder_named() {
        let cmd = CommandDefinition::new("test", "Test", "/test --arg <val>", "test.md")
            .with_args(vec![
                CommandArg::new("arg", "An argument"),
            ]);

        let bound = ArgBinder::bind_args(&cmd, &["--arg".to_string(), "value".to_string()]).unwrap();
        assert_eq!(bound.get("arg").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_variable_resolver() {
        let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
        let mut parsed_args = serde_json::Map::new();
        parsed_args.insert("name".to_string(), serde_json::Value::String("Alice".to_string()));

        let context = CommandExecutionContext::new(cmd, parsed_args)
            .with_user_input("test input");

        let resolved = VariableResolver::resolve_string("Hello {{ name }}!", &context);
        assert_eq!(resolved, "Hello Alice!");
    }

    #[test]
    fn test_variable_resolver_with_filter() {
        let cmd = CommandDefinition::new("test", "Test", "/test", "test.md");
        let mut parsed_args = serde_json::Map::new();
        parsed_args.insert("name".to_string(), serde_json::Value::String("alice".to_string()));

        let context = CommandExecutionContext::new(cmd, parsed_args);

        let resolved = VariableResolver::resolve_string("Hello {{ name | upper }}!", &context);
        assert_eq!(resolved, "Hello ALICE!");
    }
}
