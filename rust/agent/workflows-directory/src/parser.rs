//! Workflow Parser
//!
//! Parses Markdown workflow definitions.

use std::path::Path;

use crate::types::*;

/// Workflow parser for Markdown definitions
pub struct WorkflowParser;

impl WorkflowParser {
    /// Parse a workflow from a file
    pub async fn parse_file(path: &Path) -> WorkflowDirectoryResult<WorkflowDefinition> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| WorkflowDirectoryError::ParseError(format!("Failed to read file: {}", e)))?;

        Self::parse_content(&content, path)
    }

    /// Parse a workflow from content
    pub fn parse_content(content: &str, path: &Path) -> WorkflowDirectoryResult<WorkflowDefinition> {
        // Split content into lines for easier processing
        let lines: Vec<&str> = content.lines().collect();

        // Extract front matter
        let (front_matter_lines, body_lines) = Self::extract_frontmatter(&lines)?;

        // Parse front matter
        let metadata = Self::parse_frontmatter(&front_matter_lines, path)?;

        // Create workflow definition
        let mut workflow = WorkflowDefinition::new(metadata);

        // Join body back to string for section extraction
        let body = body_lines.join("\n");

        // Parse prerequisites section
        workflow.prerequisites = Self::parse_section(&body, "## 前置条件");

        // Parse parameters section
        workflow.parameters = Self::parse_parameters_section(&body);

        // Parse steps section
        workflow.steps = Self::parse_steps_section(&body)?;

        // Parse outputs section
        workflow.outputs = Self::parse_list_section(&body, "## 输出结果");

        // Parse notes section
        workflow.notes = Self::parse_list_section(&body, "## 注意事项");

        Ok(workflow)
    }

    /// Extract front matter lines from content
    fn extract_frontmatter<'a>(lines: &'a [&str]) -> WorkflowDirectoryResult<(Vec<&'a str>, Vec<&'a str>)> {
        if lines.is_empty() {
            return Err(WorkflowDirectoryError::ParseError(
                "Missing front matter (---)".to_string(),
            ));
        }

        // Skip leading empty lines to find the opening ---
        let start_idx = lines
            .iter()
            .position(|l| !l.trim().is_empty())
            .ok_or_else(|| WorkflowDirectoryError::ParseError(
                "Missing front matter (---)".to_string(),
            ))?;

        if lines[start_idx].trim() != "---" {
            return Err(WorkflowDirectoryError::ParseError(
                "Missing front matter (---)".to_string(),
            ));
        }

        // Find closing ---
        let closing_idx = lines[start_idx + 1..]
            .iter()
            .position(|l| l.trim() == "---")
            .map(|p| p + start_idx + 1)
            .ok_or_else(|| WorkflowDirectoryError::ParseError("Front matter not closed".to_string()))?;

        let front_matter: Vec<&'a str> = lines[start_idx + 1..closing_idx].to_vec();
        let body: Vec<&'a str> = lines[closing_idx + 1..].to_vec();

        Ok((front_matter, body))
    }

    /// Parse front matter into metadata
    fn parse_frontmatter(front_matter: &[&str], path: &Path) -> WorkflowDirectoryResult<WorkflowMetadata> {
        let mut name = None;
        let mut category = None;
        let mut tags: Vec<String> = Vec::new();
        let mut description = None;
        let mut author = None;
        let mut version = None;

        for line in front_matter {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "name" => name = Some(value.to_string()),
                    "category" => category = Some(value.to_string()),
                    "tags" => {
                        let tags_str = value.trim_matches(|c| c == '[' || c == ']');
                        tags = tags_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "description" => description = Some(value.to_string()),
                    "author" => author = Some(value.to_string()),
                    "version" => version = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        let name = name.ok_or_else(|| WorkflowDirectoryError::ParseError("Missing 'name' in front matter".to_string()))?;
        let category = category.unwrap_or_else(|| "uncategorized".to_string());
        let description = description.ok_or_else(|| WorkflowDirectoryError::ParseError("Missing 'description' in front matter".to_string()))?;

        let file_path = path.to_string_lossy().to_string();

        Ok(WorkflowMetadata {
            name,
            category,
            tags,
            description,
            author,
            version,
            file_path,
        })
    }

    /// Parse a section and return its content lines
    fn extract_section<'a>(content: &'a str, header: &str) -> Option<&'a str> {
        let start = content.find(header)?;
        let after_header = &content[start + header.len()..];
        // Look for next section header: "\n## " (newline followed by ##)
        let end = after_header.find("\n## ").unwrap_or(after_header.len());
        Some(&after_header[..end])
    }

    /// Parse prerequisites section
    fn parse_section(content: &str, header: &str) -> WorkflowPrerequisites {
        let mut prerequisites = WorkflowPrerequisites::default();

        if let Some(section) = Self::extract_section(content, header) {
            for line in section.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('-') || trimmed.starts_with('*') {
                    let item = trimmed.trim_start_matches('-').trim_start_matches('*').trim();
                    if !item.is_empty() && !item.starts_with('#') {
                        prerequisites.items.push(item.to_string());
                    }
                }
            }
        }

        prerequisites
    }

    /// Parse parameters section
    fn parse_parameters_section(content: &str) -> Vec<WorkflowParameter> {
        let mut parameters = Vec::new();

        if let Some(section) = Self::extract_section(content, "## 输入参数") {
            for line in section.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('|') && !trimmed.starts_with("| ---") && trimmed.contains("|") {
                    let parts: Vec<&str> = trimmed.split('|').collect();
                    if parts.len() >= 4 {
                        let name = parts[1].trim();
                        let param_type = parts[2].trim();
                        let required = parts[3].trim().contains("是") || parts[3].trim() == "true";
                        let description = parts.get(4).map(|s| s.trim()).unwrap_or("").to_string();

                        if !name.is_empty() && name != "参数" && !name.chars().all(|c| c == '-' || c == '|') {
                            parameters.push(WorkflowParameter {
                                name: name.to_string(),
                                param_type: param_type.to_string(),
                                required,
                                description,
                                default: None,
                            });
                        }
                    }
                }
            }
        }

        parameters
    }

    /// Parse steps section
    fn parse_steps_section(content: &str) -> WorkflowDirectoryResult<Vec<WorkflowStep>> {
        let mut steps = Vec::new();

        let Some(section) = Self::extract_section(content, "## 执行步骤") else {
            return Ok(steps);
        };

        let mut current_step: Option<WorkflowStep> = None;
        let mut step_name = String::new();
        let mut step_id = String::new();
        let mut agent = String::new();
        let mut prompt_lines: Vec<String> = Vec::new();
        let mut in_prompt = false;

        for line in section.lines() {
            let trimmed = line.trim();

            // Step header
            if trimmed.starts_with("### 步骤") || trimmed.starts_with("### Step") {
                // Save previous step
                if let Some(mut step) = current_step.take() {
                    if !prompt_lines.is_empty() {
                        step.prompt = prompt_lines.join("\n");
                    }
                    steps.push(step);
                    prompt_lines.clear();
                }

                // Parse step header
                let parts: Vec<&str> = trimmed.split(':').collect();
                step_id = format!("step{}", steps.len() + 1);
                step_name = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

                if step_name.is_empty() {
                    step_name = format!("Step {}", steps.len() + 1);
                }

                // Create current step immediately if we have agent info
                if !agent.is_empty() {
                    current_step = Some(WorkflowStep::new(&step_id, &step_name, &agent, ""));
                }
            } else if trimmed.starts_with("使用 **Agent") || trimmed.starts_with("使用 **agent") {
                // Parse agent
                let agent_part = trimmed
                    .replace("使用 **Agent", "")
                    .replace("使用 **agent", "")
                    .replace("** 执行：", "")
                    .replace("** 执行:", "")
                    .replace("**", "")
                    .trim()
                    .to_string();
                agent = agent_part;

                // Create current step if we have step info but no current step yet
                if current_step.is_none() && !step_name.is_empty() {
                    current_step = Some(WorkflowStep::new(&step_id, &step_name, &agent, ""));
                }
            } else if trimmed.starts_with("在 **步骤") {
                // Parse dependencies
                if let Some(mut step) = current_step.take() {
                    let dep_part = trimmed
                        .replace("在 **", "")
                        .replace("** 完成后执行：", "")
                        .replace("** 完成后执行:", "")
                        .replace("** 完成后**执行：", "")
                        .replace("** 完成后**执行:", "")
                        .trim()
                        .to_string();

                    for dep in dep_part.split(',') {
                        let dep = dep.trim().to_string();
                        if !dep.is_empty() {
                            step.depends_on.push(dep);
                        }
                    }
                    current_step = Some(step);
                }
            } else if trimmed.starts_with("```") {
                in_prompt = !in_prompt;
            } else if in_prompt {
                prompt_lines.push(trimmed.to_string());
            } else if trimmed.starts_with("输入：") || trimmed.starts_with("输入:") {
                let input_lines = trimmed[3..].trim();
                if !input_lines.is_empty() && input_lines != "-" {
                    if let Some(step) = current_step.as_mut() {
                        for input_line in input_lines.split(',') {
                            let input = input_line.trim();
                            if input.contains("来自") {
                                if let Some((name, source)) = input.split_once(':') {
                                    let source = source.replace("来自", "").trim().to_string();
                                    step.inputs.push(StepInput::new(name.trim(), &source));
                                }
                            }
                        }
                    }
                }
            } else if trimmed.starts_with("输出：") || trimmed.starts_with("输出:") {
                let output_lines = trimmed[3..].trim();
                if !output_lines.is_empty() && output_lines != "-" {
                    if let Some(step) = current_step.as_mut() {
                        for output_line in output_lines.split(',') {
                            let output = output_line.trim().replace("- ", "");
                            if !output.is_empty() {
                                let parts: Vec<&str> = output.split(':').collect();
                                let name = parts.first().map(|s| s.trim()).unwrap_or("");
                                let desc = parts.get(1).map(|s| s.trim()).unwrap_or("");
                                step.outputs.push(StepOutput::new(name, desc));
                            }
                        }
                    }
                }
            } else if !trimmed.is_empty()
                && !trimmed.starts_with("#")
                && !trimmed.starts_with("使用")
                && !trimmed.starts_with("在")
                && !trimmed.starts_with("---")
                && current_step.is_none()
            {
                // Start a new step with this as the prompt
                if !agent.is_empty() || !step_name.is_empty() {
                    current_step = Some(WorkflowStep::new(
                        &step_id,
                        &step_name,
                        &agent,
                        trimmed,
                    ));
                }
            }
        }

        // Save last step
        if let Some(mut step) = current_step {
            if !prompt_lines.is_empty() {
                step.prompt = prompt_lines.join("\n");
            }
            steps.push(step);
        }

        Ok(steps)
    }

    /// Parse a list section (outputs or notes)
    fn parse_list_section(content: &str, header: &str) -> Vec<String> {
        let mut items = Vec::new();

        if let Some(section) = Self::extract_section(content, header) {
            for line in section.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('-') || trimmed.starts_with('*') {
                    let item = trimmed.trim_start_matches('-').trim_start_matches('*').trim();
                    if !item.is_empty() && !item.starts_with('#') && !item.starts_with("##") {
                        items.push(item.to_string());
                    }
                }
            }
        }

        items
    }
}

/// Parse workflow index from README content
pub fn parse_index(content: &str) -> Vec<WorkflowIndexEntry> {
    let mut entries = Vec::new();
    let mut current_category = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Category header
        if trimmed.starts_with("### ") && !trimmed.contains("[") {
            current_category = trimmed.trim_start_matches("### ").to_string();
        }

        // Workflow entry
        if trimmed.starts_with('-') && trimmed.contains('[') && trimmed.contains("](") {
            if let Some((name_part, desc_part)) = trimmed.split_once("](") {
                let name = name_part.trim_start_matches("- [").trim();
                let desc = desc_part.trim_end_matches(')').trim();

                if !name.is_empty() && !current_category.is_empty() {
                    entries.push(WorkflowIndexEntry {
                        name: name.to_string(),
                        category: current_category.clone(),
                        description: desc.to_string(),
                        tags: Vec::new(),
                        file_path: format!("workflows/{}.md", name),
                    });
                }
            }
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let lines = vec![
            "---",
            "name: test",
            "category: testing",
            "---",
            "# Body",
        ];

        let (fm, body) = WorkflowParser::extract_frontmatter(&lines).unwrap();
        assert_eq!(fm, vec!["name: test", "category: testing"]);
        assert_eq!(body, vec!["# Body"]);
    }

    #[tokio::test]
    async fn test_parse_simple_workflow() {
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

入力：
- `param1`: 来自 {{ input1 }}

出力：
- `result1`: The result

## 输出結果

- Output 1
- Output 2
"#;

        let path = Path::new("workflows/test.md");
        let result = WorkflowParser::parse_content(content, path).unwrap();

        assert_eq!(result.metadata.name, "test-workflow");
        assert_eq!(result.metadata.category, "testing");
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].step_id, "step1");
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
    }
}
