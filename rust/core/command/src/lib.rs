//! Command Module
//!
//! Manages CLI command definitions, parsing, and execution.
//!
//! Design Reference: docs/03-module-design/core/command.md

pub mod types;
pub mod parser;
pub mod manager;

pub use types::{
    CommandError, CommandResult, CommandType, CommandMetadata, CommandArg,
    CommandUsage, WorkflowConfig, CommandDefinition, ParsedArgs,
    CommandExecutionContext, CommandExecutionResult, CommandInfo, CommandEntry,
    BuiltinFunction,
};

pub use parser::{CommandParser, ArgBinder, VariableResolver};
pub use manager::{CommandManagerImpl, CommandConfig};
