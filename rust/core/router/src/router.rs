//! Router Implementation
//!
//! Handles CLI input routing and command dispatch.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info};

use crate::types::*;

/// Router implementation
pub struct RouterImpl {
    builtin_commands: Arc<AsyncRwLock<HashMap<String, BuiltinCommand>>>,
    user_commands: Arc<AsyncRwLock<HashMap<String, UserCommand>>>,
    routes: Arc<AsyncRwLock<HashMap<String, Route>>>,
    initialized: Arc<AsyncRwLock<bool>>,
}

impl RouterImpl {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            builtin_commands: Arc::new(AsyncRwLock::new(HashMap::new())),
            user_commands: Arc::new(AsyncRwLock::new(HashMap::new())),
            routes: Arc::new(AsyncRwLock::new(HashMap::new())),
            initialized: Arc::new(AsyncRwLock::new(false)),
        }
    }

    /// Check if the router is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.try_read().map(|g| *g).unwrap_or(false)
    }

    /// Initialize the router with built-in commands
    pub async fn initialize(&self) -> RouterResult<()> {
        self.register_builtin_commands().await;
        let mut initialized = self.initialized.write().await;
        *initialized = true;
        info!("Router initialized");
        Ok(())
    }

    /// Register built-in commands
    async fn register_builtin_commands(&self) {
        let builtins = vec![
            BuiltinCommand {
                name: "help".to_string(),
                description: "Show help information".to_string(),
                aliases: vec!["?".to_string()],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::Builtin,
                    name: "help".to_string(),
                },
            },
            BuiltinCommand {
                name: "clear".to_string(),
                description: "Clear the screen".to_string(),
                aliases: vec!["cl".to_string()],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::Builtin,
                    name: "clear".to_string(),
                },
            },
            BuiltinCommand {
                name: "exit".to_string(),
                description: "Exit the application".to_string(),
                aliases: vec!["quit".to_string(), "q".to_string()],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::Builtin,
                    name: "exit".to_string(),
                },
            },
            BuiltinCommand {
                name: "status".to_string(),
                description: "Show current status".to_string(),
                aliases: vec!["stat".to_string(), "st".to_string()],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::Builtin,
                    name: "status".to_string(),
                },
            },
            BuiltinCommand {
                name: "session".to_string(),
                description: "Manage sessions".to_string(),
                aliases: vec!["sess".to_string()],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::Session,
                    name: "session".to_string(),
                },
            },
            BuiltinCommand {
                name: "agent".to_string(),
                description: "Manage agents".to_string(),
                aliases: vec![],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::Agent,
                    name: "agent".to_string(),
                },
            },
            BuiltinCommand {
                name: "history".to_string(),
                description: "Show command history".to_string(),
                aliases: vec!["hist".to_string()],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::Builtin,
                    name: "history".to_string(),
                },
            },
            BuiltinCommand {
                name: "command".to_string(),
                description: "List user commands".to_string(),
                aliases: vec!["cmd".to_string()],
                handler: CommandHandler {
                    handler_type: CommandHandlerType::CommandModule,
                    name: "command".to_string(),
                },
            },
        ];

        let mut commands = self.builtin_commands.write().await;
        for cmd in builtins {
            // Also register aliases
            commands.insert(cmd.name.clone(), cmd.clone());
            for alias in &cmd.aliases {
                commands.insert(alias.clone(), cmd.clone());
            }
        }
    }

    /// Register a user command
    pub async fn register_user_command(&self, command: UserCommand) -> RouterResult<()> {
        let mut commands = self.user_commands.write().await;
        commands.insert(command.name.clone(), command);
        info!("Registered user command");
        Ok(())
    }

    /// Handle user input
    pub async fn handle_input(&self, request: HandleInputRequest) -> HandleInputResult {
        let input = ParsedInput::new(&request.input);

        // Empty input
        if input.is_empty() {
            return HandleInputResult {
                response: RouterResponse::error("Empty input"),
                to_agent: false,
                should_exit: false,
            };
        }

        // Non-command input - forward to agent
        if !input.is_command {
            return HandleInputResult {
                response: RouterResponse::forwarded_to_agent("Forwarding to agent"),
                to_agent: true,
                should_exit: false,
            };
        }

        // Command input - find and execute
        let command_name = input.command_name.as_ref().unwrap();
        debug!("Routing command: {}", command_name);

        // Try built-in commands first
        {
            let builtins = self.builtin_commands.read().await;
            if let Some(cmd) = builtins.get(command_name) {
                return self.execute_builtin(cmd, &input, &request.session_id).await;
            }
        }

        // Try user commands
        {
            let user_cmds = self.user_commands.read().await;
            if let Some(cmd) = user_cmds.get(command_name) {
                return self.execute_user_command(cmd, &input).await;
            }
        }

        // Command not found
        HandleInputResult {
            response: RouterResponse::error(format!("Unknown command: {}", command_name)),
            to_agent: false,
            should_exit: false,
        }
    }

    /// Execute a built-in command
    async fn execute_builtin(&self, command: &BuiltinCommand, input: &ParsedInput, session_id: &str) -> HandleInputResult {
        match command.handler.name.as_str() {
            "help" => self.cmd_help(input).await,
            "clear" => self.cmd_clear().await,
            "exit" => self.cmd_exit().await,
            "status" => self.cmd_status(session_id).await,
            "session" => self.cmd_session(input).await,
            "agent" => self.cmd_agent(input).await,
            "history" => self.cmd_history().await,
            "command" => self.cmd_list_commands().await,
            _ => HandleInputResult {
                response: RouterResponse::error(format!("Unknown builtin: {}", command.handler.name)),
                to_agent: false,
                should_exit: false,
            },
        }
    }

    /// Execute a user command
    async fn execute_user_command(&self, command: &UserCommand, _input: &ParsedInput) -> HandleInputResult {
        // In a real implementation, this would invoke the command module
        HandleInputResult {
            response: RouterResponse::success_with_data(
                format!("Executing user command: {}", command.name),
                serde_json::json!({
                    "command": command.name,
                    "template": command.template,
                }),
            ),
            to_agent: false,
            should_exit: false,
        }
    }

    /// Help command
    async fn cmd_help(&self, input: &ParsedInput) -> HandleInputResult {
        let args = &input.args;

        if args.is_empty() {
            // Show general help
            let commands = self.list_commands_internal().await;
            let message = format!(
                "Available commands:\n{}\n\nType /help <command> for more info",
                commands
                    .iter()
                    .map(|c| format!("  /{} - {}", c.name, c.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            return HandleInputResult {
                response: RouterResponse::success(message),
                to_agent: false,
                should_exit: false,
            };
        }

        // Show specific command help
        let cmd_name = &args[0];
        let builtins = self.builtin_commands.read().await;
        if let Some(cmd) = builtins.get(cmd_name) {
            let aliases = if cmd.aliases.is_empty() {
                String::new()
            } else {
                format!(" (aliases: {})", cmd.aliases.join(", "))
            };
            HandleInputResult {
                response: RouterResponse::success(format!(
                    "/{} - {}{}",
                    cmd.name, cmd.description, aliases
                )),
                to_agent: false,
                should_exit: false,
            }
        } else {
            HandleInputResult {
                response: RouterResponse::error(format!("Unknown command: {}", cmd_name)),
                to_agent: false,
                should_exit: false,
            }
        }
    }

    /// Clear command
    async fn cmd_clear(&self) -> HandleInputResult {
        HandleInputResult {
            response: RouterResponse::success("Screen cleared"),
            to_agent: false,
            should_exit: false,
        }
    }

    /// Exit command
    async fn cmd_exit(&self) -> HandleInputResult {
        HandleInputResult {
            response: RouterResponse::success("Goodbye!"),
            to_agent: false,
            should_exit: true,
        }
    }

    /// Status command
    async fn cmd_status(&self, session_id: &str) -> HandleInputResult {
        HandleInputResult {
            response: RouterResponse::success_with_data(
                "Current status",
                serde_json::json!({
                    "session_id": session_id,
                    "status": "active"
                }),
            ),
            to_agent: false,
            should_exit: false,
        }
    }

    /// Session command
    async fn cmd_session(&self, input: &ParsedInput) -> HandleInputResult {
        let args = &input.args;

        if args.is_empty() {
            return HandleInputResult {
                response: RouterResponse::success("Session commands: list, new, switch, delete"),
                to_agent: false,
                should_exit: false,
            };
        }

        match args[0].as_str() {
            "list" => HandleInputResult {
                response: RouterResponse::success("Listing sessions..."),
                to_agent: true, // Delegate to session manager
                should_exit: false,
            },
            "new" => HandleInputResult {
                response: RouterResponse::success("Creating new session..."),
                to_agent: true,
                should_exit: false,
            },
            _ => HandleInputResult {
                response: RouterResponse::error("Unknown session subcommand"),
                to_agent: false,
                should_exit: false,
            },
        }
    }

    /// Agent command
    async fn cmd_agent(&self, input: &ParsedInput) -> HandleInputResult {
        let args = &input.args;

        if args.is_empty() {
            return HandleInputResult {
                response: RouterResponse::success("Agent commands: list, start, stop, switch"),
                to_agent: false,
                should_exit: false,
            };
        }

        match args[0].as_str() {
            "list" => HandleInputResult {
                response: RouterResponse::success("Listing agents..."),
                to_agent: true,
                should_exit: false,
            },
            _ => HandleInputResult {
                response: RouterResponse::error("Unknown agent subcommand"),
                to_agent: false,
                should_exit: false,
            },
        }
    }

    /// History command
    async fn cmd_history(&self) -> HandleInputResult {
        // In a real implementation, this would retrieve from history
        HandleInputResult {
            response: RouterResponse::success("Command history (recent 10)..."),
            to_agent: false,
            should_exit: false,
        }
    }

    /// List commands command
    async fn cmd_list_commands(&self) -> HandleInputResult {
        let commands = self.list_commands_internal().await;
        HandleInputResult {
            response: RouterResponse::success_with_data(
                "Available commands",
                serde_json::json!(commands),
            ),
            to_agent: false,
            should_exit: false,
        }
    }

    /// Internal method to list commands
    async fn list_commands_internal(&self) -> Vec<CommandInfo> {
        let mut result = Vec::new();

        let builtins = self.builtin_commands.read().await;
        let mut seen = std::collections::HashSet::new();
        for cmd in builtins.values() {
            if !seen.contains(&cmd.name) {
                seen.insert(cmd.name.clone());
                result.push(CommandInfo::builtin(
                    &cmd.name,
                    &cmd.description,
                    cmd.aliases.clone(),
                ));
            }
        }

        let user_cmds = self.user_commands.read().await;
        for cmd in user_cmds.values() {
            result.push(CommandInfo::user(&cmd.name, &cmd.description));
        }

        result
    }

    /// List all available commands
    pub async fn list_commands(&self, filter: Option<&str>) -> Vec<CommandInfo> {
        let all = self.list_commands_internal().await;

        match filter {
            Some("builtin") => all.into_iter().filter(|c| c.command_type == CommandType::Builtin).collect(),
            Some("user") => all.into_iter().filter(|c| c.command_type == CommandType::User).collect(),
            Some("workflow") => all.into_iter().filter(|c| c.command_type == CommandType::Workflow).collect(),
            _ => all,
        }
    }

    /// Register a route
    pub async fn register_route(&self, route: Route) -> RouterResult<()> {
        let mut routes = self.routes.write().await;
        routes.insert(route.path.clone(), route);
        Ok(())
    }

    /// Route a path
    pub async fn route(&self, path: &str) -> RouterResult<Route> {
        let routes = self.routes.read().await;
        routes
            .get(path)
            .cloned()
            .ok_or_else(|| RouterError::RouteNotFound(path.to_string()))
    }

    /// Get command info by name
    pub async fn get_command(&self, name: &str) -> Option<CommandInfo> {
        let builtins = self.builtin_commands.read().await;
        builtins.get(name).map(|cmd| {
            CommandInfo::builtin(&cmd.name, &cmd.description, cmd.aliases.clone())
        })
    }

    /// Check if input is a command
    pub fn is_command(input: &str) -> bool {
        input.trim().starts_with('/')
    }

    /// Clear all user commands
    pub async fn clear_user_commands(&self) {
        let mut commands = self.user_commands.write().await;
        commands.clear();
    }

    /// Get total command count
    pub async fn command_count(&self) -> usize {
        let builtins = self.builtin_commands.read().await;
        let user_cmds = self.user_commands.read().await;
        builtins.len() + user_cmds.len()
    }
}

impl Default for RouterImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::RouterHandle for RouterImpl {
    async fn handle_input(&self, input: String, session_id: String) -> HandleInputResult {
        RouterImpl::handle_input(
            self,
            HandleInputRequest { input, session_id },
        )
        .await
    }

    fn is_initialized(&self) -> bool {
        RouterImpl::is_initialized(self)
    }
}
