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
