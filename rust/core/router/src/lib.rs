//! Router
//!
//! CLI input routing and command dispatch.
//!
//! Design Reference: docs/03-module-design/core/router.md

pub mod router;
pub mod types;

pub use router::RouterImpl;
pub use types::*;

/// RouterHandle trait for external consumers (TUI, CLI)
#[async_trait::async_trait]
pub trait RouterHandle: Send + Sync {
    /// Handle user input and return result
    async fn handle_input(&self, input: String, session_id: String) -> HandleInputResult;

    /// Check if the router is initialized
    fn is_initialized(&self) -> bool;
}
