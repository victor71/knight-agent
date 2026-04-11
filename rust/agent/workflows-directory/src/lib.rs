//! Workflows Directory
//!
//! Manages workflow definitions loaded from Markdown files.
//!
//! Design Reference: docs/03-module-design/agent/workflows-directory.md

pub mod manager;
pub mod parser;
pub mod types;

pub use manager::WorkflowDirectoryImpl;
pub use parser::{parse_index, WorkflowParser};
pub use types::*;

// Re-export for backwards compatibility
pub use types::WorkflowDirectory;
