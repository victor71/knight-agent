//! Tool System
//!
//! Unified tool framework providing tool registration, validation, and execution.
//! Supports built-in tools, custom tools, and MCP tools.
//!
//! Design Reference: docs/03-module-design/tools/tool-system.md

mod builtins;
mod registry;
mod types;
mod validator;

pub use types::*;
pub use registry::ToolRegistry;
pub use validator::ArgumentValidator;

use async_trait::async_trait;
use builtins::{all_builtins, builtin_names, BuiltinTool};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Tool system errors
#[derive(Error, Debug)]
pub enum ToolSystemError {
    #[error("Tool system not initialized")]
    NotInitialized,
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Tool already exists: {0}")]
    AlreadyExists(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Timeout executing tool: {0}")]
    Timeout(String),
}

/// Result type for tool system operations
pub type ToolResult<T> = Result<T, ToolSystemError>;

/// Tool system trait
#[async_trait]
pub trait ToolSystemTrait: Send + Sync {
    /// Create a new tool system instance
    fn new() -> Result<Self, ToolSystemError>
    where
        Self: Sized;

    /// Get the name of this tool system
    fn name(&self) -> &str;

    /// Check if the tool system is initialized
    fn is_initialized(&self) -> bool;

    /// Register a tool
    async fn register_tool(&self, tool: ToolDefinition) -> ToolResult<()>;

    /// Unregister a tool
    async fn unregister_tool(&self, name: &str) -> ToolResult<()>;

    /// List all available tools
    async fn list_tools(&self) -> ToolResult<Vec<ToolInfo>>;

    /// List tools by category
    async fn list_tools_by_category(&self, category: &str) -> ToolResult<Vec<ToolInfo>>;

    /// Get tool information
    async fn get_tool(&self, name: &str) -> ToolResult<Option<ToolInfo>>;

    /// Execute a tool
    async fn execute(&self, request: ExecuteRequest) -> ToolResult<ToolExecutionResult>;

    /// Validate tool arguments
    async fn validate_args(
        &self,
        name: &str,
        args: &serde_json::Value,
    ) -> ToolResult<ValidationResult>;

    /// Get all categories
    async fn get_categories(&self) -> ToolResult<Vec<String>>;

    /// Register MCP tools from a server
    async fn register_mcp_tools(
        &self,
        server_name: &str,
        tools: Vec<McpToolDefinition>,
    ) -> ToolResult<usize>;
}

/// Tool system implementation
pub struct ToolSystemImpl {
    registry: Arc<RwLock<ToolRegistry>>,
    builtin_tools: HashMap<String, BuiltinTool>,
    initialized: bool,
}

impl ToolSystemImpl {
    /// Create a new tool system with built-in tools registered
    pub fn new() -> Result<Self, ToolSystemError> {
        let registry = Arc::new(RwLock::new(ToolRegistry::new()));
        let mut builtin_tools: HashMap<String, BuiltinTool> = HashMap::new();

        // Register built-in tools
        for tool in all_builtins() {
            let name = tool.name().to_string();
            debug!("Registering built-in tool: {}", name);
            builtin_tools.insert(name, tool);
        }

        Ok(Self {
            registry,
            builtin_tools,
            initialized: true,
        })
    }

    /// Create a new tool system without built-in tools
    pub fn empty() -> Result<Self, ToolSystemError> {
        Ok(Self {
            registry: Arc::new(RwLock::new(ToolRegistry::new())),
            builtin_tools: HashMap::new(),
            initialized: true,
        })
    }

    /// Execute a built-in tool
    async fn execute_builtin(
        &self,
        name: &str,
        args: &serde_json::Value,
        context: &ToolContext,
    ) -> ToolResult<ToolExecutionResult> {
        let start = Instant::now();

        let tool = match self.builtin_tools.get(name) {
            Some(t) => t,
            None => {
                return Err(ToolSystemError::NotFound(name.to_string()));
            }
        };

        debug!(
            "Executing built-in tool: {} with args: {:?}",
            name, args
        );

        let result = tool.execute(args, context).await;
        let duration = start.elapsed().as_millis() as u64;

        if result.success {
            info!("Tool {} executed successfully in {}ms", name, duration);
            Ok(result.with_duration(duration))
        } else {
            warn!("Tool {} failed in {}ms: {:?}", name, duration, result.error);
            Ok(result.with_duration(duration))
        }
    }

    /// Execute a registered tool (custom tool with command handler)
    async fn execute_registered(
        &self,
        name: &str,
        args: &serde_json::Value,
        context: &ToolContext,
    ) -> ToolResult<ToolExecutionResult> {
        let start = Instant::now();

        let registry = self.registry.read().await;
        let tool = match registry.get(name) {
            Some(t) => t.clone(),
            None => {
                return Err(ToolSystemError::NotFound(name.to_string()));
            }
        };
        drop(registry);

        debug!("Executing registered tool: {}", name);

        // For command type tools, execute the command
        match tool.handler.handler_type {
            HandlerType::Command => {
                // Build command from target and args
                let target = &tool.handler.target;

                // Substitute template variables
                let mut cmd_str = target.to_string();
                if let Some(obj) = args.as_object() {
                    for (key, value) in obj {
                        let placeholder = format!("{{{{{}}}}}", key);
                        let value_str = match value {
                            serde_json::Value::String(s) => s.clone(),
                            _ => value.to_string(),
                        };
                        cmd_str = cmd_str.replace(&placeholder, &value_str);
                    }
                }

                info!("Executing command: {}", cmd_str);

                let output = tokio::process::Command::new("bash")
                    .arg("-c")
                    .arg(&cmd_str)
                    .current_dir(&context.workspace)
                    .output()
                    .await
                    .map_err(|e| {
                        error!("Failed to execute command: {}", e);
                        ToolSystemError::ExecutionFailed(format!(
                            "Command execution failed: {}",
                            e
                        ))
                    })?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let duration = start.elapsed().as_millis() as u64;
                let result = serde_json::json!({
                    "stdout": stdout.to_string(),
                    "stderr": stderr.to_string(),
                    "exit_code": output.status.code().unwrap_or(-1),
                    "success": output.status.success(),
                });

                if output.status.success() {
                    Ok(ToolExecutionResult::success(result).with_duration(duration))
                } else {
                    Ok(ToolExecutionResult::error("EXECUTION_FAILED", &stderr)
                        .with_duration(duration))
                }
            }
            HandlerType::Builtin => {
                // Built-in tools are handled separately
                Err(ToolSystemError::NotFound(name.to_string()))
            }
            HandlerType::Skill | HandlerType::Mcp | HandlerType::Wasm => {
                // These are not yet implemented
                Err(ToolSystemError::ExecutionFailed(format!(
                    "Handler type {:?} not yet implemented",
                    tool.handler.handler_type
                )))
            }
        }
    }
}

impl Default for ToolSystemImpl {
    fn default() -> Self {
        Self::new().expect("Failed to create tool system")
    }
}

#[async_trait]
impl ToolSystemTrait for ToolSystemImpl {
    fn new() -> Result<Self, ToolSystemError> {
        Self::new()
    }

    fn name(&self) -> &str {
        "tool-system"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn register_tool(&self, tool: ToolDefinition) -> ToolResult<()> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let mut registry = self.registry.write().await;
        if registry.contains(&tool.name) {
            return Err(ToolSystemError::AlreadyExists(tool.name));
        }

        let name = tool.name.clone();
        registry.register(tool);
        info!("Registered tool: {}", name);
        Ok(())
    }

    async fn unregister_tool(&self, name: &str) -> ToolResult<()> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let mut registry = self.registry.write().await;
        if registry.unregister(name).is_none() {
            return Err(ToolSystemError::NotFound(name.to_string()));
        }

        info!("Unregistered tool: {}", name);
        Ok(())
    }

    async fn list_tools(&self) -> ToolResult<Vec<ToolInfo>> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let registry = self.registry.read().await;
        let mut tools = registry.list_info();

        // Add built-in tools
        for (name, tool) in &self.builtin_tools {
            if registry.to_info(name).is_none() {
                // Add built-in tool info if not overridden by custom tool
                tools.push(ToolInfo {
                    name: name.clone(),
                    display_name: name.clone(),
                    description: format!("Built-in tool: {}", name),
                    category: "builtin".to_string(),
                    parameters: Default::default(),
                    dangerous: name == "bash" || name == "write" || name == "edit",
                    is_read_only: tool.is_read_only(),
                });
            }
        }

        Ok(tools)
    }

    async fn list_tools_by_category(
        &self,
        category: &str,
    ) -> ToolResult<Vec<ToolInfo>> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let registry = self.registry.read().await;
        let tools: Vec<ToolInfo> = registry
            .list_by_category(category)
            .into_iter()
            .map(|t| ToolInfo {
                name: t.name.clone(),
                display_name: t.display_name.clone(),
                description: t.description.clone(),
                category: t.category.clone(),
                parameters: t.parameters.clone(),
                dangerous: t.dangerous,
                is_read_only: t.is_read_only,
            })
            .collect();

        Ok(tools)
    }

    async fn get_tool(&self, name: &str) -> ToolResult<Option<ToolInfo>> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let registry = self.registry.read().await;

        // Check custom tools first
        if let Some(info) = registry.to_info(name) {
            return Ok(Some(info));
        }

        // Check built-in tools
        if let Some(tool) = self.builtin_tools.get(name) {
            return Ok(Some(ToolInfo {
                name: name.to_string(),
                display_name: name.to_string(),
                description: format!("Built-in tool: {}", name),
                category: "builtin".to_string(),
                parameters: Default::default(),
                dangerous: name == "bash" || name == "write" || name == "edit",
                is_read_only: tool.is_read_only(),
            }));
        }

        Ok(None)
    }

    async fn execute(&self, request: ExecuteRequest) -> ToolResult<ToolExecutionResult> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let name = &request.name;
        let args = &request.args;
        let context = &request.context;

        // Check if it's a built-in tool
        if self.builtin_tools.contains_key(name) {
            return self.execute_builtin(name, args, context).await;
        }

        // Check if it's a registered custom tool
        let registry = self.registry.read().await;
        if registry.contains(name) {
            drop(registry);
            return self.execute_registered(name, args, context).await;
        }

        Err(ToolSystemError::NotFound(name.to_string()))
    }

    async fn validate_args(
        &self,
        name: &str,
        args: &serde_json::Value,
    ) -> ToolResult<ValidationResult> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let registry = self.registry.read().await;

        // Check custom tools
        if let Some(tool) = registry.get(name) {
            let result = ArgumentValidator::validate(args, &tool.parameters);
            return Ok(result);
        }

        // Built-in tools use simple validation
        if builtin_names().contains(&name) {
            // Basic required field check for built-ins
            let mut result = ValidationResult::valid();
            match name {
                "read" | "write" | "edit" => {
                    if args.get("file_path").is_none() {
                        result.add_error("file_path", "required field is missing");
                    }
                }
                "grep" => {
                    if args.get("pattern").is_none() {
                        result.add_error("pattern", "required field is missing");
                    }
                }
                "glob" => {
                    if args.get("pattern").is_none() {
                        result.add_error("pattern", "required field is missing");
                    }
                }
                "bash" => {
                    if args.get("command").is_none() {
                        result.add_error("command", "required field is missing");
                    }
                }
                _ => {}
            }
            return Ok(result);
        }

        Err(ToolSystemError::NotFound(name.to_string()))
    }

    async fn get_categories(&self) -> ToolResult<Vec<String>> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let registry = self.registry.read().await;
        let mut categories = registry.categories();

        // Add built-in category if any built-ins exist
        if !self.builtin_tools.is_empty() && !categories.contains(&"builtin".to_string()) {
            categories.insert(0, "builtin".to_string());
        }

        Ok(categories)
    }

    async fn register_mcp_tools(
        &self,
        server_name: &str,
        tools: Vec<McpToolDefinition>,
    ) -> ToolResult<usize> {
        if !self.is_initialized() {
            return Err(ToolSystemError::NotInitialized);
        }

        let mut registry = self.registry.write().await;
        let count = registry.register_mcp_tools(server_name, tools);
        info!(
            "Registered {} MCP tools from server: {}",
            count, server_name
        );
        Ok(count)
    }
}
