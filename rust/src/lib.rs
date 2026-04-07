//! Knight Agent - Rust Workspace
//!
//! A multi-agent system with 26 modules organized in 7 categories:
//! - Shared Libraries (1 module)
//! - Core (8 modules)
//! - Agent (6 modules)
//! - Services (7 modules)
//! - Tool System (1 module)
//! - Infrastructure (1 module)
//! - Security (2 modules)

// Re-export knight-config for all modules
pub use knight_config;

pub mod core;
pub mod agent;
pub mod services;
pub mod tool_system;
pub mod infrastructure;
pub mod security;
