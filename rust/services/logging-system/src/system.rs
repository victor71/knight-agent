//! Logging System Implementation
//!
//! Core implementation of the logging system.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;

use tokio::sync::RwLock;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::output::LogOutput;
use crate::types::{ErrorInfo, ExportFormat, LogContext, LogEntry, LogFilter, LogLevel, LogStats};
use crate::LoggingError;

/// Logging system trait
#[allow(async_fn_in_trait)]
pub trait LoggingSystem: Send + Sync {
    fn new() -> Result<Self, LoggingError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn log(&self, entry: LogEntry) -> Result<(), LoggingError>;
    async fn get_logs(&self, filter: LogFilter) -> Result<Vec<LogEntry>, LoggingError>;
}

/// Main logging system implementation
pub struct LoggingSystemImpl {
    name: String,
    initialized: AtomicBool,
    global_level: RwLock<LogLevel>,
    module_levels: RwLock<HashMap<String, LogLevel>>,
    log_buffer: Arc<RwLock<Vec<LogEntry>>>,
    max_buffer_size: usize,
    outputs: RwLock<Vec<LogOutput>>,
}

impl LoggingSystemImpl {
    /// Create a new logging system instance
    pub fn new() -> Result<Self, LoggingError> {
        let mut module_levels = HashMap::new();
        module_levels.insert("logging_system".to_string(), LogLevel::Debug);

        Ok(Self {
            name: "logging-system".to_string(),
            initialized: AtomicBool::new(false),
            global_level: RwLock::new(LogLevel::Info),
            module_levels: RwLock::new(module_levels),
            log_buffer: Arc::new(RwLock::new(Vec::new())),
            max_buffer_size: 10000,
            outputs: RwLock::new(vec![LogOutput::Console { colored: true }]),
        })
    }

    /// Create with console output
    pub fn with_console(mut self, colored: bool) -> Self {
        self.outputs = RwLock::new(vec![LogOutput::Console { colored }]);
        self
    }

    /// Create with file output
    pub fn with_file(mut self, path: PathBuf, max_size: usize, max_files: usize) -> Self {
        self.outputs = RwLock::new(vec![LogOutput::File {
            path: path.clone(),
            rotation_max_size: max_size,
            rotation_max_files: max_files,
            compress: true,
        }]);
        self
    }

    /// Initialize the logging system
    pub async fn init(&self) -> Result<(), LoggingError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Use OnceLock to ensure tracing is only initialized once globally
        static TRACING_INIT: OnceLock<()> = OnceLock::new();

        let _ = TRACING_INIT.get_or_init(|| {
            let env_filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_ansi(true);

            let registry = tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer);

            let _ = registry.try_init();
        });

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Set the global log level
    pub async fn set_level(&self, level: LogLevel) -> Result<(), LoggingError> {
        let mut global = self.global_level.write().await;
        *global = level;
        Ok(())
    }

    /// Get the current global log level
    pub async fn get_level(&self) -> LogLevel {
        let global = self.global_level.read().await;
        *global
    }

    /// Set log level for a specific module
    pub async fn set_module_level(
        &self,
        module: String,
        level: LogLevel,
    ) -> Result<(), LoggingError> {
        let mut levels = self.module_levels.write().await;
        levels.insert(module, level);
        Ok(())
    }

    /// Get log level for a specific module
    pub async fn get_module_level(&self, module: &str) -> Option<LogLevel> {
        let levels = self.module_levels.read().await;
        levels.get(module).cloned()
    }

    /// Check if an entry should be logged based on level filtering
    async fn should_log(&self, entry: &LogEntry) -> bool {
        // Hold both locks to avoid race condition between module and global level reads
        let module_levels = self.module_levels.read().await;
        let global_level = self.global_level.read().await;

        let effective_level = module_levels
            .get(&entry.module)
            .copied()
            .unwrap_or(*global_level);

        entry.level >= effective_level
    }

    /// Log a message with specified level
    pub async fn log_message(
        &self,
        level: LogLevel,
        message: String,
        context: Option<LogContext>,
        module: Option<String>,
        session_id: Option<String>,
        error: Option<ErrorInfo>,
    ) -> Result<(), LoggingError> {
        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            level,
            module: module.unwrap_or_else(|| "unknown".to_string()),
            session_id,
            message,
            context: context.map(|c| c.fields).unwrap_or_default(),
            error,
        };

        self.log(entry).await
    }

    /// Log at debug level
    pub async fn debug(&self, message: String) -> Result<(), LoggingError> {
        self.log_message(LogLevel::Debug, message, None, None, None, None)
            .await
    }

    /// Log at info level
    pub async fn info(&self, message: String) -> Result<(), LoggingError> {
        self.log_message(LogLevel::Info, message, None, None, None, None)
            .await
    }

    /// Log at warn level
    pub async fn warn(&self, message: String) -> Result<(), LoggingError> {
        self.log_message(LogLevel::Warn, message, None, None, None, None)
            .await
    }

    /// Log at error level
    pub async fn error(&self, message: String, err: Option<ErrorInfo>) -> Result<(), LoggingError> {
        self.log_message(LogLevel::Error, message, None, None, None, err)
            .await
    }

    /// Log at fatal level
    pub async fn fatal(&self, message: String, err: Option<ErrorInfo>) -> Result<(), LoggingError> {
        self.log_message(LogLevel::Fatal, message, None, None, None, err)
            .await
    }

    /// Search logs by query string
    pub async fn search(
        &self,
        query: String,
        start_time: Option<std::time::SystemTime>,
        end_time: Option<std::time::SystemTime>,
        level: Option<LogLevel>,
        limit: Option<usize>,
    ) -> Result<Vec<LogEntry>, LoggingError> {
        let filter = LogFilter {
            level,
            module: None,
            session_id: None,
            since: start_time,
            until: end_time,
            message_pattern: Some(query),
        };

        let mut results = self.get_logs(filter).await?;

        let limit = limit.unwrap_or(100);
        if results.len() > limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Export logs in specified format
    pub async fn export(
        &self,
        format: ExportFormat,
        filter: LogFilter,
    ) -> Result<String, LoggingError> {
        let logs = self.get_logs(filter).await?;

        match format {
            ExportFormat::Json => serde_json::to_string_pretty(&logs)
                .map_err(|e| LoggingError::ExportFailed(e.to_string())),
            ExportFormat::Csv => {
                let mut csv = String::from("id,timestamp,level,module,message\n");
                for entry in logs {
                    csv.push_str(&format!(
                        "{},{},{},{},{}\n",
                        entry.id,
                        humantime::format_rfc3339_seconds(entry.timestamp),
                        entry.level,
                        entry.module,
                        entry.message.replace(',', ";")
                    ));
                }
                Ok(csv)
            }
            ExportFormat::Text => {
                let mut text = String::new();
                for entry in logs {
                    text.push_str(&format!(
                        "[{}] {} [{}] {}",
                        humantime::format_rfc3339_seconds(entry.timestamp),
                        entry.level,
                        entry.module,
                        entry.message
                    ));
                    if !entry.context.is_empty() {
                        text.push_str(&format!(" {:?}", entry.context));
                    }
                    text.push('\n');
                }
                Ok(text)
            }
        }
    }

    /// Trigger log rotation
    pub async fn rotate(&self) -> Result<(), LoggingError> {
        let outputs = self.outputs.read().await;

        for output in outputs.iter() {
            if let LogOutput::File {
                path,
                rotation_max_files,
                ..
            } = output
            {
                // Check if file exists and needs rotation
                if let Ok(metadata) = tokio::fs::metadata(path).await {
                    let _file_size = metadata.len() as usize;

                    // If file exceeds max size, rotate it
                    // Note: rotation_max_size check would be done here in full implementation
                    // For now, just verify we can access the file
                    if metadata.len() > 0 {
                        tracing::info!("Log file {} has {} bytes", path.display(), metadata.len());
                    }
                }

                // Return error if rotation_max_files is 0 (not allowed)
                if *rotation_max_files == 0 {
                    return Err(LoggingError::RotationFailed(
                        "rotation_max_files cannot be 0".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Clear all logs from buffer
    pub async fn clear(&self) -> Result<(), LoggingError> {
        let mut buffer = self.log_buffer.write().await;
        buffer.clear();
        Ok(())
    }

    /// Get log statistics
    pub async fn get_stats(&self) -> LogStats {
        let buffer = self.log_buffer.read().await;

        let total_entries = buffer.len();
        let mut entries_by_level: HashMap<String, usize> = HashMap::new();
        let mut entries_by_module: HashMap<String, usize> = HashMap::new();
        let mut oldest_entry: Option<std::time::SystemTime> = None;
        let mut newest_entry: Option<std::time::SystemTime> = None;

        for entry in buffer.iter() {
            *entries_by_level.entry(entry.level.to_string()).or_insert(0) += 1;
            *entries_by_module.entry(entry.module.clone()).or_insert(0) += 1;

            match oldest_entry {
                None => oldest_entry = Some(entry.timestamp),
                Some(ts) if entry.timestamp < ts => oldest_entry = Some(entry.timestamp),
                _ => {}
            }

            match newest_entry {
                None => newest_entry = Some(entry.timestamp),
                Some(ts) if entry.timestamp > ts => newest_entry = Some(entry.timestamp),
                _ => {}
            }
        }

        LogStats {
            total_entries,
            entries_by_level,
            entries_by_module,
            oldest_entry,
            newest_entry,
        }
    }

    /// Write log entry to console
    async fn write_to_console(&self, entry: &LogEntry, colored: bool) {
        let level_str = match entry.level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        };

        let timestamp = humantime::format_rfc3339_seconds(entry.timestamp);

        if colored {
            let color = match entry.level {
                LogLevel::Trace | LogLevel::Debug => "\x1b[36m", // cyan
                LogLevel::Info => "\x1b[32m",                    // green
                LogLevel::Warn => "\x1b[33m",                    // yellow
                LogLevel::Error | LogLevel::Fatal => "\x1b[31m", // red
            };
            let reset = "\x1b[0m";

            eprintln!(
                "{}{} [{}] [{}]{} {}",
                color, timestamp, level_str, entry.module, reset, entry.message
            );
        } else {
            eprintln!(
                "{} [{}] [{}] {}",
                timestamp, level_str, entry.module, entry.message
            );
        }
    }

    /// Write log entry to file
    async fn write_to_file(&self, entry: &LogEntry, path: &PathBuf) -> Result<(), LoggingError> {
        use tokio::io::AsyncWriteExt;

        let json =
            serde_json::to_string(entry).map_err(|e| LoggingError::WriteFailed(e.to_string()))?;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| LoggingError::WriteFailed(e.to_string()))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| LoggingError::WriteFailed(e.to_string()))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| LoggingError::WriteFailed(e.to_string()))?;

        Ok(())
    }
}

impl LoggingSystem for LoggingSystemImpl {
    fn new() -> Result<Self, LoggingError> {
        Self::new()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    async fn log(&self, entry: LogEntry) -> Result<(), LoggingError> {
        if !self.is_initialized() {
            return Err(LoggingError::NotInitialized);
        }

        if !self.should_log(&entry).await {
            return Ok(());
        }

        // Store in buffer
        {
            let mut buffer = self.log_buffer.write().await;
            if buffer.len() >= self.max_buffer_size {
                buffer.remove(0);
            }
            buffer.push(entry.clone());
        }

        // Write to outputs
        let outputs = self.outputs.read().await;
        for output in outputs.iter() {
            match output {
                LogOutput::Console { colored } => {
                    self.write_to_console(&entry, *colored).await;
                }
                LogOutput::File { path, .. } => {
                    self.write_to_file(&entry, path).await?;
                }
            }
        }

        Ok(())
    }

    async fn get_logs(&self, filter: LogFilter) -> Result<Vec<LogEntry>, LoggingError> {
        let buffer = self.log_buffer.read().await;

        // Pre-compute lowercase pattern once to avoid repeated allocations
        let pattern_lower = filter.message_pattern.as_ref().map(|p| p.to_lowercase());

        let filtered: Vec<LogEntry> = buffer
            .iter()
            .filter(|entry| {
                if let Some(level) = &filter.level {
                    if entry.level < *level {
                        return false;
                    }
                }
                if let Some(module) = &filter.module {
                    if &entry.module != module {
                        return false;
                    }
                }
                if let Some(session_id) = &filter.session_id {
                    if entry.session_id.as_ref() != Some(session_id) {
                        return false;
                    }
                }
                if let Some(since) = filter.since {
                    if entry.timestamp < since {
                        return false;
                    }
                }
                if let Some(until) = filter.until {
                    if entry.timestamp > until {
                        return false;
                    }
                }
                if let Some(ref pattern) = pattern_lower {
                    if !entry.message.to_lowercase().contains(pattern) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        Ok(filtered)
    }
}
