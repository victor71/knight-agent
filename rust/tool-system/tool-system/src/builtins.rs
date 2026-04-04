//! Built-in Tools
//!
//! Implementation of built-in tools: read, write, edit, grep, glob, bash.

use crate::types::{ToolContext, ToolExecutionResult};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info};

/// Enum for all built-in tools
#[derive(Debug, Clone)]
pub enum BuiltinTool {
    Read(ReadTool),
    Write(WriteTool),
    Edit(EditTool),
    Grep(GrepTool),
    Glob(GlobTool),
    Bash(BashTool),
}

impl BuiltinTool {
    /// Get the tool name
    pub fn name(&self) -> &str {
        match self {
            BuiltinTool::Read(_) => "read",
            BuiltinTool::Write(_) => "write",
            BuiltinTool::Edit(_) => "edit",
            BuiltinTool::Grep(_) => "grep",
            BuiltinTool::Glob(_) => "glob",
            BuiltinTool::Bash(_) => "bash",
        }
    }

    /// Check if this tool is read-only (can be executed in parallel)
    pub fn is_read_only(&self) -> bool {
        match self {
            BuiltinTool::Read(_) => true,
            BuiltinTool::Write(_) => false,
            BuiltinTool::Edit(_) => false,
            BuiltinTool::Grep(_) => true,
            BuiltinTool::Glob(_) => true,
            BuiltinTool::Bash(_) => false,
        }
    }

    /// Execute the tool
    pub async fn execute(&self, args: &Value, context: &ToolContext) -> ToolExecutionResult {
        match self {
            BuiltinTool::Read(tool) => tool.execute(args, context).await,
            BuiltinTool::Write(tool) => tool.execute(args, context).await,
            BuiltinTool::Edit(tool) => tool.execute(args, context).await,
            BuiltinTool::Grep(tool) => tool.execute(args, context).await,
            BuiltinTool::Glob(tool) => tool.execute(args, context).await,
            BuiltinTool::Bash(tool) => tool.execute(args, context).await,
        }
    }
}

/// Read file tool
#[derive(Debug, Clone, Default)]
pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, args: &Value, _context: &ToolContext) -> ToolExecutionResult {
        let start = Instant::now();
        let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "file_path is required");
            }
        };

        debug!("Reading file: {}", file_path);

        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(u64::MAX) as usize;

        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().skip(offset).take(limit).collect();
                let result = serde_json::json!({
                    "content": lines.join("\n"),
                    "lines_read": lines.len(),
                    "total_lines": content.lines().count(),
                    "file_path": file_path,
                });
                ToolExecutionResult::success(result)
                    .with_duration(start.elapsed().as_millis() as u64)
            }
            Err(e) => {
                error!("Failed to read file {}: {}", file_path, e);
                ToolExecutionResult::error("EXECUTION_FAILED", &format!("Failed to read file: {}", e))
                    .with_duration(start.elapsed().as_millis() as u64)
            }
        }
    }
}

/// Write file tool
#[derive(Debug, Clone, Default)]
pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, args: &Value, _context: &ToolContext) -> ToolExecutionResult {
        let start = Instant::now();
        let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "file_path is required");
            }
        };

        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "content is required");
            }
        };

        debug!("Writing file: {} ({} bytes)", file_path, content.len());

        // Ensure parent directory exists
        if let Some(parent) = Path::new(file_path).parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    error!("Failed to create directory: {}", e);
                    return ToolExecutionResult::error(
                        "EXECUTION_FAILED",
                        &format!("Failed to create directory: {}", e),
                    )
                    .with_duration(start.elapsed().as_millis() as u64);
                }
            }
        }

        match tokio::fs::write(file_path, content).await {
            Ok(()) => {
                info!("Successfully wrote file: {}", file_path);
                let result = serde_json::json!({
                    "file_path": file_path,
                    "bytes_written": content.len(),
                });
                ToolExecutionResult::success(result)
                    .with_duration(start.elapsed().as_millis() as u64)
            }
            Err(e) => {
                error!("Failed to write file {}: {}", file_path, e);
                ToolExecutionResult::error("EXECUTION_FAILED", &format!("Failed to write file: {}", e))
                    .with_duration(start.elapsed().as_millis() as u64)
            }
        }
    }
}

/// Edit file tool (replace old_string with new_string)
#[derive(Debug, Clone, Default)]
pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, args: &Value, _context: &ToolContext) -> ToolExecutionResult {
        let start = Instant::now();
        let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "file_path is required");
            }
        };

        let old_string = match args.get("old_string").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "old_string is required");
            }
        };

        let new_string = match args.get("new_string").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "new_string is required");
            }
        };

        debug!(
            "Editing file: {} (replacing '{}')",
            file_path, old_string
        );

        // Read file content
        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read file {}: {}", file_path, e);
                return ToolExecutionResult::error(
                    "EXECUTION_FAILED",
                    &format!("Failed to read file: {}", e),
                )
                .with_duration(start.elapsed().as_millis() as u64);
            }
        };

        // Check if old_string exists
        if !content.contains(old_string) {
            return ToolExecutionResult::error("INVALID_ARGS", "old_string not found in file")
                .with_duration(start.elapsed().as_millis() as u64);
        }

        // Replace
        let new_content = content.replace(old_string, new_string);

        // Write back
        match tokio::fs::write(file_path, &new_content).await {
            Ok(()) => {
                info!("Successfully edited file: {}", file_path);
                let result = serde_json::json!({
                    "file_path": file_path,
                    "replacements": 1,
                });
                ToolExecutionResult::success(result)
                    .with_duration(start.elapsed().as_millis() as u64)
            }
            Err(e) => {
                error!("Failed to write file {}: {}", file_path, e);
                ToolExecutionResult::error("EXECUTION_FAILED", &format!("Failed to write file: {}", e))
                    .with_duration(start.elapsed().as_millis() as u64)
            }
        }
    }
}

/// Grep tool (search file contents)
#[derive(Debug, Clone, Default)]
pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, args: &Value, context: &ToolContext) -> ToolExecutionResult {
        let start = Instant::now();
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "pattern is required");
            }
        };

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&context.workspace);

        debug!("Grep: pattern='{}' path='{}'", pattern, path);

        // Build grep command
        let mut cmd = tokio::process::Command::new("grep");
        cmd.arg("-n"); // Line numbers
        cmd.arg("-r"); // Recursive
        cmd.arg(pattern);
        cmd.arg(path);

        // Add glob filter if provided
        if let Some(glob) = args.get("glob").and_then(|v| v.as_str()) {
            cmd.arg("--include").arg(glob);
        }

        let output = match cmd.output().await {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to execute grep: {}", e);
                return ToolExecutionResult::error(
                    "EXECUTION_FAILED",
                    &format!("Failed to execute grep: {}", e),
                )
                .with_duration(start.elapsed().as_millis() as u64);
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && !stderr.is_empty() {
            return ToolExecutionResult::error("EXECUTION_FAILED", &format!("grep error: {}", stderr))
                .with_duration(start.elapsed().as_millis() as u64);
        }

        let matches: Vec<&str> = stdout.lines().collect();
        let result = serde_json::json!({
            "matches": matches,
            "count": matches.len(),
            "pattern": pattern,
            "path": path,
        });

        ToolExecutionResult::success(result).with_duration(start.elapsed().as_millis() as u64)
    }
}

/// Glob tool (find files by pattern)
#[derive(Debug, Clone, Default)]
pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, args: &Value, context: &ToolContext) -> ToolExecutionResult {
        let start = Instant::now();
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "pattern is required");
            }
        };

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&context.workspace);

        debug!("Glob: pattern='{}' path='{}'", pattern, path);

        let output = tokio::process::Command::new("find")
            .arg(path)
            .arg("-type")
            .arg("f")
            .arg("-name")
            .arg(pattern)
            .output()
            .await;

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let files: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
                let result = serde_json::json!({
                    "files": files,
                    "count": files.len(),
                    "pattern": pattern,
                    "path": path,
                });
                ToolExecutionResult::success(result)
                    .with_duration(start.elapsed().as_millis() as u64)
            }
            Err(e) => {
                error!("Failed to execute glob: {}", e);
                ToolExecutionResult::error("EXECUTION_FAILED", &format!("Failed to execute glob: {}", e))
                    .with_duration(start.elapsed().as_millis() as u64)
            }
        }
    }
}

/// Bash tool (execute shell commands)
#[derive(Debug, Clone, Default)]
pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, args: &Value, context: &ToolContext) -> ToolExecutionResult {
        let start = Instant::now();
        let command = match args.get("command").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => {
                return ToolExecutionResult::error("INVALID_ARGS", "command is required");
            }
        };

        let _timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        debug!("Bash: {} (timeout={}s)", command, _timeout_secs);

        // For Windows, use cmd.exe
        let output = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(&context.workspace)
            .output()
            .await;

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                let result = serde_json::json!({
                    "stdout": stdout.to_string(),
                    "stderr": stderr.to_string(),
                    "exit_code": o.status.code().unwrap_or(-1),
                    "success": o.status.success(),
                });
                ToolExecutionResult::success(result)
                    .with_duration(start.elapsed().as_millis() as u64)
            }
            Err(e) => {
                error!("Failed to execute bash: {}", e);
                ToolExecutionResult::error("EXECUTION_FAILED", &format!("Failed to execute bash: {}", e))
                    .with_duration(start.elapsed().as_millis() as u64)
            }
        }
    }
}

/// Get all built-in tools as an enum
pub fn all_builtins() -> Vec<BuiltinTool> {
    vec![
        BuiltinTool::Read(ReadTool::new()),
        BuiltinTool::Write(WriteTool::new()),
        BuiltinTool::Edit(EditTool::new()),
        BuiltinTool::Grep(GrepTool::new()),
        BuiltinTool::Glob(GlobTool::new()),
        BuiltinTool::Bash(BashTool::new()),
    ]
}

/// Get built-in tool names
pub fn builtin_names() -> Vec<&'static str> {
    vec!["read", "write", "edit", "grep", "glob", "bash"]
}
