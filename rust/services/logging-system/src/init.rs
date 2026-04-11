//! Logger Guard
//!
//! Global logger initialization utilities.

use tracing_subscriber::{
    fmt::{self},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Guard for initializing the global logger
pub struct LoggerGuard;

impl LoggerGuard {
    /// Initialize the global logger with default settings
    pub fn init() -> Result<(), crate::LoggingError> {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(true);

        let registry = tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer);

        let _ = registry.try_init();

        Ok(())
    }
}
