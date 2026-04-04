//! Workflows Directory
//!
//! Manages workflow definitions loaded from Markdown files.
//!
//! Design Reference: docs/03-module-design/agent/workflows-directory.md

pub mod manager;
pub mod parser;
pub mod types;

pub use manager::WorkflowDirectoryImpl;
pub use types::*;
pub use parser::{WorkflowParser, parse_index};

// Re-export for backwards compatibility
pub use types::WorkflowDirectory;
