//! Monitor
//!
//! System monitoring and metrics collection.
//!
//! Design Reference: docs/03-module-design/core/monitor.md

pub mod monitor;
pub mod types;

pub use monitor::MonitorImpl;
pub use types::*;
