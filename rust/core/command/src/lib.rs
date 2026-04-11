//! Command Module
//!
//! Manages CLI command definitions, parsing, and execution.
//!
//! Design Reference: docs/03-module-design/core/command.md

pub mod manager;
pub mod parser;
pub mod types;

pub use types::{
    BuiltinFunction, CommandArg, CommandDefinition, CommandEntry, CommandError,
    CommandExecutionContext, CommandExecutionResult, CommandInfo, CommandMetadata, CommandResult,
    CommandType, CommandUsage, ParsedArgs, WorkflowConfig,
};

pub use manager::{CommandConfig, CommandManagerImpl};
pub use parser::{ArgBinder, CommandParser, VariableResolver};
