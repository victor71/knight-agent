//! Router
//!
//! CLI input routing and command dispatch.
//!
//! Design Reference: docs/03-module-design/core/router.md

pub mod types;
pub mod router;

pub use types::*;
pub use router::RouterImpl;
