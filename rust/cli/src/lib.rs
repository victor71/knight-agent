//! CLI (Command Line Interface)
//!
//! Command-line interface for Knight-Agent.
//! Provides REPL mode and daemon control.
//!
//! Design Reference: docs/03-module-design/cli/cli.md

// Re-export public API
pub use cli_impl::CliImpl;
pub use error::{CliError, CliResult};
pub use repl::{CliRepl, ReplState};
pub use r#trait::Cli;
pub use types::{DaemonAction, ReplCommand, ReplInput};

mod cli_impl;
mod error;
mod repl;
mod r#trait;
mod types;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_input_parse() {
        let empty = ReplInput::parse("");
        assert!(matches!(empty, ReplInput::Empty));

        let slash = ReplInput::parse("/help");
        assert!(matches!(slash, ReplInput::SlashCommand { command: c, args: a } if c == "help" && a.is_empty()));

        let slash_with_args = ReplInput::parse("/session new");
        assert!(matches!(slash_with_args, ReplInput::SlashCommand { command: c, args: a } if c == "session" && a == "new"));

        let natural = ReplInput::parse("Hello world");
        assert!(matches!(natural, ReplInput::NaturalLanguage { text } if text == "Hello world"));
    }

    #[test]
    fn test_repl_command_parse() {
        let help = ReplCommand::parse("help", "");
        assert!(matches!(help, ReplCommand::Help));

        let session_list = ReplCommand::parse("sessions", "");
        assert!(matches!(session_list, ReplCommand::SessionList));

        let session_new = ReplCommand::parse("sessions", "new test");
        assert!(matches!(session_new, ReplCommand::SessionCreate { name } if name == Some("test".to_string())));
    }

    #[tokio::test]
    async fn test_cli_new() {
        let cli = CliImpl::new().unwrap();
        assert_eq!(cli.name(), "cli");
        assert!(!cli.is_initialized());
    }

    #[tokio::test]
    async fn test_cli_initialize() {
        let cli = CliImpl::new().unwrap();
        cli.initialize().await.unwrap();
        assert!(cli.is_initialized());
    }

    #[tokio::test]
    async fn test_repl_state() {
        let repl = CliRepl::new();
        assert_eq!(repl.state().await, ReplState::Running);
    }

    #[tokio::test]
    async fn test_process_input() {
        let repl = CliRepl::new();
        let cmd = repl.process_input("/help").await.unwrap();
        assert!(matches!(cmd, ReplCommand::Help));

        let cmd = repl.process_input("test").await.unwrap();
        assert!(matches!(cmd, ReplCommand::Status)); // Natural language defaults to status
    }

    #[tokio::test]
    async fn test_execute_command() {
        let repl = CliRepl::new();
        repl.execute_command(ReplCommand::Help).await.unwrap();
        // Should print help, which we can't test directly
    }

    #[tokio::test]
    async fn test_execute_exit_command() {
        let repl = CliRepl::new();
        repl.execute_command(ReplCommand::Exit).await.unwrap();
        assert!(!repl.is_running().await);
        assert_eq!(repl.state().await, ReplState::Exiting);
    }
}
