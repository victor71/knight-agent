//! Command Manager
//!
//! Manages command registry and execution.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info, warn};

use crate::parser::{ArgBinder, CommandParser};
use crate::types::*;

/// Command Manager implementation
pub struct CommandManagerImpl {
    /// Registered commands (command_name -> CommandEntry)
    commands: Arc<AsyncRwLock<HashMap<String, CommandEntry>>>,
    /// Command directories to scan
    command_dirs: Arc<Mutex<Vec<PathBuf>>>,
    /// Configuration
    #[allow(dead_code)]
    config: Arc<Mutex<CommandConfig>>,
}

impl CommandManagerImpl {
    /// Create a new command manager
    pub fn new() -> Self {
        Self {
            commands: Arc::new(AsyncRwLock::new(HashMap::new())),
            command_dirs: Arc::new(Mutex::new(Vec::new())),
            config: Arc::new(Mutex::new(CommandConfig::default())),
        }
    }

    /// Create with custom config
    pub fn with_config(config: CommandConfig) -> Self {
        Self {
            commands: Arc::new(AsyncRwLock::new(HashMap::new())),
            command_dirs: Arc::new(Mutex::new(Vec::new())),
            config: Arc::new(Mutex::new(config)),
        }
    }

    // ========== Command Registration ==========

    /// Register a command definition
    pub async fn register_command(&self, definition: CommandDefinition) -> CommandResult<()> {
        let mut commands = self.commands.write().await;
        let name = definition.metadata.name.clone();

        // Check if command already exists
        if commands.contains_key(&name) {
            return Err(CommandError::InvalidDefinition(
                format!("Command '{}' already registered", name),
            ));
        }

        // Validate command definition
        self.validate_definition(&definition)?;

        commands.insert(name.clone(), CommandEntry::new(definition));
        info!("Registered command: {}", name);
        Ok(())
    }

    /// Unregister a command
    pub async fn unregister_command(&self, name: &str) -> CommandResult<()> {
        let mut commands = self.commands.write().await;
        commands
            .remove(name)
            .ok_or_else(|| CommandError::NotFound(name.to_string()))?;
        info!("Unregistered command: {}", name);
        Ok(())
    }

    /// Enable a command
    pub async fn enable_command(&self, name: &str) -> CommandResult<()> {
        let mut commands = self.commands.write().await;
        if let Some(entry) = commands.get_mut(name) {
            entry.enabled = true;
            Ok(())
        } else {
            Err(CommandError::NotFound(name.to_string()))
        }
    }

    /// Disable a command
    pub async fn disable_command(&self, name: &str) -> CommandResult<()> {
        let mut commands = self.commands.write().await;
        if let Some(entry) = commands.get_mut(name) {
            entry.enabled = false;
            Ok(())
        } else {
            Err(CommandError::NotFound(name.to_string()))
        }
    }

    // ========== Command Lookup ==========

    /// Get a command definition
    pub async fn get_command(&self, name: &str) -> CommandResult<CommandDefinition> {
        let commands = self.commands.read().await;
        commands
            .get(name)
            .map(|e| e.definition.clone())
            .ok_or_else(|| CommandError::NotFound(name.to_string()))
    }

    /// List all registered commands
    pub async fn list_commands(&self) -> Vec<CommandInfo> {
        let commands = self.commands.read().await;
        commands
            .values()
            .filter(|e| e.enabled)
            .map(|e| CommandInfo::from_definition(&e.definition))
            .collect()
    }

    /// List all commands (including disabled)
    pub async fn list_all_commands(&self) -> Vec<CommandInfo> {
        let commands = self.commands.read().await;
        commands
            .values()
            .map(|e| CommandInfo::from_definition(&e.definition))
            .collect()
    }

    /// Check if a command exists
    pub async fn has_command(&self, name: &str) -> bool {
        let commands = self.commands.read().await;
        commands.contains_key(name)
    }

    // ========== Command Execution ==========

    /// Execute a command by name with raw input
    pub async fn execute_command(
        &self,
        name: &str,
        user_input: &str,
        session_id: Option<&str>,
    ) -> CommandResult<CommandExecutionResult> {
        let start_time = std::time::Instant::now();

        // Get command definition
        let definition = self.get_command(name).await?;

        // Parse arguments from user input
        let parsed_args = self.parse_arguments(&definition, user_input)?;

        // Build execution context
        let mut context = CommandExecutionContext::new(definition.clone(), parsed_args);
        context.user_input = user_input.to_string();
        if let Some(sid) = session_id {
            context.session_id = Some(sid.to_string());
        }

        // Execute based on command type
        let result = match definition.metadata.command_type {
            CommandType::Simple => {
                self.execute_simple_command(&definition, &context).await
            }
            CommandType::Workflow => {
                self.execute_workflow_command(&definition, &context).await
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;
        Ok(result?.with_execution_time(execution_time))
    }

    /// Execute a simple command
    async fn execute_simple_command(
        &self,
        definition: &CommandDefinition,
        context: &CommandExecutionContext,
    ) -> CommandResult<CommandExecutionResult> {
        // For simple commands, we return info about what would be executed
        // The actual LLM-driven execution would be handled by integration with other modules
        let mut output = format!(
            "Command: {}\nDescription: {}\n",
            definition.metadata.name, definition.metadata.description
        );

        if !context.parsed_args.is_empty() {
            output.push_str("Arguments:\n");
            for (key, value) in &context.parsed_args {
                output.push_str(&format!("  {}: {}\n", key, value));
            }
        }

        if let Some(ref behavior) = definition.usage.expected_behavior {
            output.push_str(&format!("\nExpected behavior:\n{}\n", behavior));
        }

        Ok(CommandExecutionResult::success(&output))
    }

    /// Execute a workflow command
    async fn execute_workflow_command(
        &self,
        definition: &CommandDefinition,
        context: &CommandExecutionContext,
    ) -> CommandResult<CommandExecutionResult> {
        let workflow_name = context
            .parsed_args
            .get("workflow_name")
            .or_else(|| context.parsed_args.get("0"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let output = format!(
            "Workflow command: {}\nWorkflow name: {}\n\n\
            Note: Workflow execution requires Task Manager integration.\n\
            Workflow would be executed with the following arguments: {:?}",
            definition.metadata.name,
            workflow_name,
            context.parsed_args
        );

        Ok(CommandExecutionResult::success(&output))
    }

    /// Parse arguments from user input string
    fn parse_arguments(
        &self,
        definition: &CommandDefinition,
        user_input: &str,
    ) -> CommandResult<ParsedArgs> {
        // Remove command name from input
        let input = if let Some(stripped) = user_input.strip_prefix('/') {
            if let Some(space_idx) = stripped.find(|c: char| c.is_whitespace()) {
                &stripped[space_idx + 1..]
            } else {
                ""
            }
        } else {
            user_input.trim()
        };

        // Split into args
        let args: Vec<String> = if input.is_empty() {
            Vec::new()
        } else {
            input.split_whitespace().map(|s| s.to_string()).collect()
        };

        // Bind arguments using ArgBinder
        ArgBinder::bind_args(definition, &args)
    }

    // ========== Command Loading ==========

    /// Add a command directory to scan
    pub async fn add_command_dir(&self, path: PathBuf) {
        let mut dirs = self.command_dirs.lock().unwrap();
        if !dirs.contains(&path) {
            dirs.push(path);
        }
    }

    /// Load commands from registered directories
    pub async fn load_commands_from_dirs(&self) -> CommandResult<usize> {
        let dirs = self.command_dirs.lock().unwrap().clone();
        let mut loaded = 0;

        for dir in dirs {
            match self.load_commands_from_dir(&dir).await {
                Ok(count) => loaded += count,
                Err(e) => {
                    warn!("Failed to load commands from {:?}: {}", dir, e);
                }
            }
        }

        Ok(loaded)
    }

    /// Load commands from a specific directory
    pub async fn load_commands_from_dir(&self, path: &PathBuf) -> CommandResult<usize> {
        let mut count = 0;

        if !path.exists() {
            return Ok(0);
        }

        let mut entries = tokio::fs::read_dir(path).await.map_err(|e| {
            CommandError::ParseError(format!("Failed to read directory: {}", e))
        })?;

        loop {
            match entries.next_entry().await {
                Ok(Some(entry)) => {
                    let file_path = entry.path();
                    if file_path.extension().and_then(|s| s.to_str()) == Some("md") {
                        match CommandParser::parse_file(&file_path).await {
                            Ok(def) => {
                                if self.register_command(def).await.is_ok() {
                                    count += 1;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse command file {:?}: {}", file_path, e);
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    break;
                }
            }
        }

        debug!("Loaded {} commands from {:?}", count, path);
        Ok(count)
    }

    // ========== Validation ==========

    /// Validate a command definition
    fn validate_definition(&self, definition: &CommandDefinition) -> CommandResult<()> {
        // Check name is valid
        if definition.metadata.name.is_empty() {
            return Err(CommandError::InvalidDefinition("Command name cannot be empty".to_string()));
        }

        if definition.metadata.name.contains(' ') {
            return Err(CommandError::InvalidDefinition(
                "Command name cannot contain spaces".to_string(),
            ));
        }

        // Check syntax is valid
        if definition.usage.syntax.is_empty() {
            return Err(CommandError::InvalidDefinition("Command syntax cannot be empty".to_string()));
        }

        // Check for required args without defaults
        for arg in &definition.args {
            if arg.required && arg.default.is_none() {
                // This is actually fine - the arg binding will handle the error
            }
        }

        // Check workflow commands have workflow config
        if definition.metadata.command_type == CommandType::Workflow
            && definition.workflow_config.is_none() {
                // Not necessarily an error - could be set dynamically
            }

        Ok(())
    }

    // ========== Statistics ==========

    /// Get command count
    pub async fn command_count(&self) -> usize {
        let commands = self.commands.read().await;
        commands.len()
    }

    /// Get enabled command count
    pub async fn enabled_command_count(&self) -> usize {
        let commands = self.commands.read().await;
        commands.values().filter(|e| e.enabled).count()
    }
}

impl Default for CommandManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Command configuration
#[derive(Debug, Clone)]
pub struct CommandConfig {
    pub hot_reload: bool,
    pub validate_on_load: bool,
    pub max_commands: usize,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            hot_reload: false,
            validate_on_load: true,
            max_commands: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_command() {
        let cm = CommandManagerImpl::new();

        let cmd = CommandDefinition::new("test", "Test command", "/test <arg>", "test.md")
            .with_args(vec![CommandArg::new("arg", "An argument").with_required(true)]);

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

        let result = cm.execute_command("hello", "/hello Alice", None).await.unwrap();
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
}
