//! CLI type definitions

use serde::{Deserialize, Serialize};

/// Daemon action
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DaemonAction {
    Start,
    Stop,
    Status,
    Restart,
}

/// REPL input type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplInput {
    SlashCommand { command: String, args: String },
    NaturalLanguage { text: String },
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplCommand {
    // Help
    Help,
    // Session management
    SessionList,
    SessionCreate { name: Option<String> },
    SessionSwitch { id: String },
    SessionDestroy { id: String },
    // Agent management
    AgentList,
    AgentSpawn { variant: String },
    // System
    Status,
    Quit,
    Exit,
    // Unknown
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
