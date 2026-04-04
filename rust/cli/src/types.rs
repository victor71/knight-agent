//! CLI type definitions

use serde::{Deserialize, Serialize};

/// Daemon control actions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DaemonAction {
    /// Start the daemon process
    Start,
    /// Stop the daemon process
    Stop,
    /// Query daemon status
    Status,
    /// Restart the daemon process
    Restart,
}

/// Parsed REPL input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplInput {
    /// Slash command (e.g., "/help", "/sessions")
    SlashCommand { command: String, args: String },
    /// Natural language input
    NaturalLanguage { text: String },
    /// Empty input
    Empty,
}

impl ReplInput {
    /// Parse a line of input
    pub fn parse(line: &str) -> Self {
        let line = line.trim();
        if line.is_empty() {
            return ReplInput::Empty;
        }
        if line.starts_with('/') {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            let command = parts[0].trim_start_matches('/').to_string();
            let args = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
            return ReplInput::SlashCommand { command, args };
        }
        ReplInput::NaturalLanguage {
            text: line.to_string(),
        }
    }
}

/// REPL command type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReplCommand {
    /// Show help information
    Help,
    /// List all sessions
    SessionList,
    /// Create a new session
    SessionCreate { name: Option<String> },
    /// Switch to a session
    SessionSwitch { id: String },
    /// Destroy a session
    SessionDestroy { id: String },
    /// List all agents
    AgentList,
    /// Spawn a new agent
    AgentSpawn { variant: String },
    /// Show system status
    Status,
    /// Exit the REPL
    Quit,
    /// Exit the REPL (alias for Quit)
    Exit,
    /// Unknown command
    Unknown { command: String },
}

impl ReplCommand {
    /// Parse command string
    pub fn parse(command: &str, args: &str) -> Self {
        match command {
            "h" | "help" => ReplCommand::Help,
            "session" | "sessions" => {
                if args.is_empty() {
                    ReplCommand::SessionList
                } else {
                    let parts: Vec<&str> = args.splitn(2, ' ').collect();
                    match parts.first() {
                        Some(&"new" | &"create") => ReplCommand::SessionCreate {
                            name: parts.get(1).map(|s| s.to_string()),
                        },
                        Some(&"switch") => ReplCommand::SessionSwitch {
                            id: parts.get(1).unwrap_or(&"").to_string(),
                        },
                        Some(&"destroy" | &"rm") => ReplCommand::SessionDestroy {
                            id: parts.get(1).unwrap_or(&"").to_string(),
                        },
                        _ => ReplCommand::SessionList,
                    }
                }
            }
            "agent" | "agents" => {
                if args.is_empty() {
                    ReplCommand::AgentList
                } else {
                    let parts: Vec<&str> = args.splitn(2, ' ').collect();
                    match parts.first() {
                        Some(&"spawn" | &"new") => ReplCommand::AgentSpawn {
                            variant: parts.get(1).unwrap_or(&"").to_string(),
                        },
                        _ => ReplCommand::AgentList,
                    }
                }
            }
            "status" => ReplCommand::Status,
            "quit" | "exit" => ReplCommand::Exit,
            _ => ReplCommand::Unknown {
                command: command.to_string(),
            },
        }
    }
}
