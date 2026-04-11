//! Hook Engine
//!
//! Hook registration, lookup, management, and execution.
//!
//! Design Reference: docs/03-module-design/core/hook-engine.md

pub mod executor;
pub mod registry;
pub mod types;

pub use executor::HookExecutor;
pub use registry::HookRegistry;
pub use types::*;
