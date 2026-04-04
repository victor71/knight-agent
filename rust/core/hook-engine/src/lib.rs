//! Hook Engine
//!
//! Hook registration, lookup, management, and execution.
//!
//! Design Reference: docs/03-module-design/core/hook-engine.md

pub mod types;
pub mod registry;
pub mod executor;

pub use registry::HookRegistry;
pub use executor::HookExecutor;
pub use types::*;
