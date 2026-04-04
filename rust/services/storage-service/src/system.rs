//! Storage Service Implementation
//!
//! Main implementation of the storage service.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

use crate::database::{Database, StorageError};
use crate::types::*;

/// Storage service trait
#[allow(async_fn_in_trait)]
pub trait StorageService: Send + Sync {
    fn new() -> Result<Self, StorageError>
    where
        Self: Sized;

    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;

    // Session operations
    async fn save_session(&self, session: Session) -> Result<bool, StorageError>;
    async fn load_session(&self, session_id: &str) -> Result<Option<Session>, StorageError>;
    async fn delete_session(&self, session_id: &str) -> Result<bool, StorageError>;
    async fn list_sessions(
        &self,
        filter: SessionFilter,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Session>, StorageError>;

    // Message operations
    async fn append_message(&self, message: Message) -> Result<bool, StorageError>;
    async fn get_messages(
        &self,
        session_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        after: Option<&str>,
    ) -> Result<Vec<Message>, StorageError>;
    async fn delete_messages(&self, session_id: &str, before: &str) -> Result<i64, StorageError>;

    // Compression point operations
    async fn save_compression_point(&self, point: CompressionPoint) -> Result<bool, StorageError>;
    async fn get_compression_points(&self, session_id: &str) -> Result<Vec<CompressionPoint>, StorageError>;
    async fn delete_compression_point(&self, point_id: &str) -> Result<bool, StorageError>;

    // Task operations
    async fn save_task(&self, task: Task) -> Result<bool, StorageError>;
    async fn load_task(&self, task_id: &str) -> Result<Option<Task>, StorageError>;
    async fn update_task(&self, task_id: &str, updates: TaskUpdate) -> Result<bool, StorageError>;
    async fn list_tasks(&self, filter: TaskFilter, limit: Option<usize>) -> Result<Vec<Task>, StorageError>;

    // Workflow operations
    async fn save_workflow(&self, workflow: WorkflowDefinition) -> Result<bool, StorageError>;
    async fn load_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>, StorageError>;
    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>, StorageError>;

    // Config operations
    async fn save_config(&self, key: &str, value: serde_json::Value) -> Result<bool, StorageError>;
    async fn load_config(&self, key: &str) -> Result<Option<serde_json::Value>, StorageError>;
    async fn delete_config(&self, key: &str) -> Result<bool, StorageError>;

    // Statistics
    async fn get_stats(&self) -> Result<StorageStats, StorageError>;

    // Statistics persistence
    async fn save_stats_snapshot(&self, snapshot: StatsSnapshot) -> Result<bool, StorageError>;
    async fn query_stats_range(
        &self,
        start_time: i64,
        end_time: i64,
        granularity: Option<&str>,
    ) -> Result<Vec<StatsSnapshot>, StorageError>;
    async fn save_token_usage(&self, usage: TokenUsageRecord) -> Result<bool, StorageError>;
    async fn save_llm_call(&self, call: LLMCallRecord) -> Result<bool, StorageError>;
    async fn save_session_event(&self, event: SessionEvent) -> Result<bool, StorageError>;

    // Report
    async fn get_daily_report(&self, date: &str) -> Result<Option<DailyReport>, StorageError>;

    // Backup and restore
    async fn backup(&self, path: &str) -> Result<bool, StorageError>;
    async fn restore(&self, path: &str) -> Result<bool, StorageError>;
    async fn export_data(&self, format: &str, output_path: &str) -> Result<bool, StorageError>;

    // Maintenance
    async fn vacuum(&self) -> Result<i64, StorageError>;
    async fn reindex(&self) -> Result<bool, StorageError>;
}

/// Main storage service implementation
pub struct StorageServiceImpl {
    name: String,
    initialized: AtomicBool,
    config: RwLock<StorageConfig>,
    db: Arc<Database>,
}

impl StorageServiceImpl {
    /// Create a new storage service instance
    pub fn new() -> Result<Self, StorageError> {
        Self::with_config(StorageConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: StorageConfig) -> Result<Self, StorageError> {
        let db_path = PathBuf::from(&config.database_path);

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| StorageError::InvalidData(format!("failed to create directory: {}", e)))?;
        }

        let db = Database::open(&db_path)?;

        Ok(Self {
            name: "storage-service".to_string(),
            initialized: AtomicBool::new(false),
            config: RwLock::new(config),
            db: Arc::new(db),
        })
    }

    /// Initialize the storage service
    pub async fn init(&self) -> Result<(), StorageError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Initializing storage service");
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl StorageService for StorageServiceImpl {
    fn new() -> Result<Self, StorageError> {
        Self::new()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    // =============================================================================
    // Session Operations
    // =============================================================================

    async fn save_session(&self, session: Session) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.save_session(&session)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn load_session(&self, session_id: &str) -> Result<Option<Session>, StorageError> {
        let db = Arc::clone(&self.db);
        let session_id = session_id.to_string();
        tokio::task::spawn_blocking(move || db.load_session(&session_id))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn delete_session(&self, session_id: &str) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        let session_id = session_id.to_string();
        tokio::task::spawn_blocking(move || db.delete_session(&session_id))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn list_sessions(
        &self,
        filter: SessionFilter,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Session>, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.list_sessions(&filter, limit, offset))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    // =============================================================================
    // Message Operations
    // =============================================================================

    async fn append_message(&self, message: Message) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.append_message(&message)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn get_messages(
        &self,
        session_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        after: Option<&str>,
    ) -> Result<Vec<Message>, StorageError> {
        let db = Arc::clone(&self.db);
        let session_id = session_id.to_string();
        let after = after.map(|s| s.to_string());
        tokio::task::spawn_blocking(move || db.get_messages(&session_id, limit, offset, after.as_deref()))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn delete_messages(&self, session_id: &str, before: &str) -> Result<i64, StorageError> {
        let db = Arc::clone(&self.db);
        let session_id = session_id.to_string();
        let before = before.to_string();
        tokio::task::spawn_blocking(move || db.delete_messages_before(&session_id, &before))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    // =============================================================================
    // Compression Point Operations
    // =============================================================================

    async fn save_compression_point(&self, point: CompressionPoint) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.save_compression_point(&point)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn get_compression_points(&self, session_id: &str) -> Result<Vec<CompressionPoint>, StorageError> {
        let db = Arc::clone(&self.db);
        let session_id = session_id.to_string();
        tokio::task::spawn_blocking(move || db.get_compression_points(&session_id))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn delete_compression_point(&self, point_id: &str) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        let point_id = point_id.to_string();
        tokio::task::spawn_blocking(move || db.delete_compression_point(&point_id))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    // =============================================================================
    // Task Operations
    // =============================================================================

    async fn save_task(&self, task: Task) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.save_task(&task)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn load_task(&self, task_id: &str) -> Result<Option<Task>, StorageError> {
        let db = Arc::clone(&self.db);
        let task_id = task_id.to_string();
        tokio::task::spawn_blocking(move || db.load_task(&task_id))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn update_task(&self, task_id: &str, updates: TaskUpdate) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        let task_id = task_id.to_string();
        tokio::task::spawn_blocking(move || db.update_task(&task_id, &updates))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn list_tasks(&self, filter: TaskFilter, limit: Option<usize>) -> Result<Vec<Task>, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.list_tasks(&filter, limit))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    // =============================================================================
    // Workflow Operations
    // =============================================================================

    async fn save_workflow(&self, workflow: WorkflowDefinition) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.save_workflow(&workflow)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn load_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>, StorageError> {
        let db = Arc::clone(&self.db);
        let workflow_id = workflow_id.to_string();
        tokio::task::spawn_blocking(move || db.load_workflow(&workflow_id))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.list_workflows())
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    // =============================================================================
    // Config Operations
    // =============================================================================

    async fn save_config(&self, key: &str, value: serde_json::Value) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_string();
        let value_str = serde_json::to_string(&value)
            .map_err(|e| StorageError::InvalidData(e.to_string()))?;
        tokio::task::spawn_blocking(move || {
            db.save_config(&key, &value_str)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn load_config(&self, key: &str) -> Result<Option<serde_json::Value>, StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            if let Some(value_str) = db.load_config(&key)? {
                let value: serde_json::Value = serde_json::from_str(&value_str)
                    .map_err(|e| StorageError::InvalidData(e.to_string()))?;
                Ok::<_, StorageError>(Some(value))
            } else {
                Ok::<_, StorageError>(None)
            }
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn delete_config(&self, key: &str) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        let key = key.to_string();
        tokio::task::spawn_blocking(move || db.delete_config(&key))
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    // =============================================================================
    // Statistics
    // =============================================================================

    async fn get_stats(&self) -> Result<StorageStats, StorageError> {
        let db_size = self.db.get_database_size()?;
        let db_size_mb = db_size as f64 / (1024.0 * 1024.0);

        Ok(StorageStats {
            sessions: SessionStats::default(),
            messages: MessageStats::default(),
            tasks: TaskStats::default(),
            database_size_mb: db_size_mb,
            compression_ratio: 0.0,
        })
    }

    // =============================================================================
    // Statistics Persistence
    // =============================================================================

    async fn save_stats_snapshot(&self, _snapshot: StatsSnapshot) -> Result<bool, StorageError> {
        // Not fully implemented
        Ok(true)
    }

    async fn query_stats_range(
        &self,
        _start_time: i64,
        _end_time: i64,
        _granularity: Option<&str>,
    ) -> Result<Vec<StatsSnapshot>, StorageError> {
        Ok(Vec::new())
    }

    async fn save_token_usage(&self, usage: TokenUsageRecord) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.save_token_usage(&usage)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn save_llm_call(&self, call: LLMCallRecord) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.save_llm_call(&call)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn save_session_event(&self, event: SessionEvent) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.save_session_event(&event)?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    // =============================================================================
    // Report
    // =============================================================================

    async fn get_daily_report(&self, _date: &str) -> Result<Option<DailyReport>, StorageError> {
        Ok(None)
    }

    // =============================================================================
    // Backup and Restore
    // =============================================================================

    async fn backup(&self, path: &str) -> Result<bool, StorageError> {
        let config = self.config.read().await;
        let source = std::path::PathBuf::from(&config.database_path);
        let dest = std::path::PathBuf::from(path);

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| StorageError::WriteFailed(format!("failed to create backup dir: {}", e)))?;
        }

        std::fs::copy(&source, &dest)
            .map_err(|e| StorageError::WriteFailed(format!("backup failed: {}", e)))?;

        info!("Backup created at {}", path);
        Ok(true)
    }

    async fn restore(&self, path: &str) -> Result<bool, StorageError> {
        let config = self.config.read().await;
        let source = std::path::PathBuf::from(path);
        let dest = std::path::PathBuf::from(&config.database_path);

        std::fs::copy(&source, &dest)
            .map_err(|e| StorageError::WriteFailed(format!("restore failed: {}", e)))?;

        info!("Restored from {}", path);
        Ok(true)
    }

    async fn export_data(&self, format: &str, output_path: &str) -> Result<bool, StorageError> {
        let sessions = self.list_sessions(SessionFilter::default(), None, None).await?;
        let data = serde_json::to_string_pretty(&sessions)
            .map_err(|e| StorageError::InvalidData(e.to_string()))?;

        match format {
            "json" | "yaml" => {
                std::fs::write(output_path, data)
                    .map_err(|e| StorageError::WriteFailed(format!("export failed: {}", e)))?;
            }
            _ => {
                return Err(StorageError::InvalidData(format!(
                    "unsupported format: {}",
                    format
                )));
            }
        }

        info!("Data exported to {}", output_path);
        Ok(true)
    }

    // =============================================================================
    // Maintenance
    // =============================================================================

    async fn vacuum(&self) -> Result<i64, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.vacuum())
            .await
            .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }

    async fn reindex(&self) -> Result<bool, StorageError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.reindex()?;
            Ok::<_, StorageError>(true)
        })
        .await
        .map_err(|e| StorageError::Database(format!("task error: {}", e)))?
    }
}

impl Default for StorageServiceImpl {
    fn default() -> Self {
        Self::new().expect("failed to create storage service")
    }
}
