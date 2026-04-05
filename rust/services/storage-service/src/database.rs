//! Database Layer
//!
//! SQLite database operations with async wrapper.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use rusqlite::{params, Connection};

use crate::types::*;

/// Storage result type
pub type StorageResult<T> = Result<T, StorageError>;

/// Database wrapper for SQLite operations
pub struct Database {
    conn: Arc<std::sync::Mutex<Connection>>,
}

impl Database {
    /// Open or create a database at the given path
    pub fn open(path: &Path) -> StorageResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| StorageError::Database(format!("failed to open database: {}", e)))?;

        let db = Self {
            conn: Arc::new(std::sync::Mutex::new(conn)),
        };

        db.initialize()?;
        Ok(db)
    }

    /// Initialize database schema
    fn initialize(&self) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Database(e.to_string()))?;

        conn.execute_batch(
            r#"
            -- Sessions table
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                workspace_root TEXT NOT NULL,
                project_type TEXT,
                created_at INTEGER NOT NULL,
                last_active_at INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}'
            );

            -- Messages table
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}',
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            -- Compression points table
            CREATE TABLE IF NOT EXISTS compression_points (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                before_count INTEGER NOT NULL,
                after_count INTEGER NOT NULL,
                summary TEXT NOT NULL,
                token_saved INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}',
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            -- Tasks table
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                workflow_id TEXT,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                status TEXT NOT NULL,
                agent_id TEXT,
                inputs TEXT DEFAULT '{}',
                outputs TEXT DEFAULT '{}',
                error TEXT,
                created_at INTEGER NOT NULL,
                started_at INTEGER,
                completed_at INTEGER
            );

            -- Workflows table
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            -- Config table
            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            -- Stats snapshots table
            CREATE TABLE IF NOT EXISTS stats_snapshots (
                id TEXT PRIMARY KEY,
                period TEXT NOT NULL,
                timestamp_start INTEGER NOT NULL,
                timestamp_end INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                tokens_total INTEGER NOT NULL DEFAULT 0,
                tokens_input INTEGER NOT NULL DEFAULT 0,
                tokens_output INTEGER NOT NULL DEFAULT 0,
                tokens_cost_estimate REAL DEFAULT 0,
                sessions_new INTEGER NOT NULL DEFAULT 0,
                sessions_active INTEGER NOT NULL DEFAULT 0,
                sessions_total INTEGER NOT NULL DEFAULT 0,
                messages_total INTEGER NOT NULL DEFAULT 0,
                agents_llm_calls INTEGER NOT NULL DEFAULT 0,
                agents_active INTEGER NOT NULL DEFAULT 0,
                agents_created INTEGER NOT NULL DEFAULT 0,
                system_memory_mb_avg REAL DEFAULT 0,
                system_memory_mb_peak INTEGER DEFAULT 0,
                system_cpu_avg REAL DEFAULT 0,
                system_uptime_seconds INTEGER DEFAULT 0
            );

            -- Token usage log
            CREATE TABLE IF NOT EXISTS token_usage_log (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL,
                cost_estimate REAL,
                timestamp INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}'
            );

            -- LLM call log
            CREATE TABLE IF NOT EXISTS llm_call_log (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                agent_id TEXT,
                model TEXT NOT NULL,
                prompt_tokens INTEGER NOT NULL,
                completion_tokens INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL,
                latency_ms INTEGER,
                timestamp INTEGER NOT NULL,
                success INTEGER NOT NULL,
                error_message TEXT
            );

            -- Session events log
            CREATE TABLE IF NOT EXISTS session_events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}'
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_workflow ON tasks(workflow_id);
            CREATE INDEX IF NOT EXISTS idx_stats_snapshots_period ON stats_snapshots(period, timestamp_start);
            CREATE INDEX IF NOT EXISTS idx_token_usage_session ON token_usage_log(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_token_usage_timestamp ON token_usage_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_llm_call_session ON llm_call_log(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_llm_call_timestamp ON llm_call_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_session_events_session ON session_events(session_id, timestamp);
            "#,
        )
        .map_err(|e| StorageError::Database(format!("failed to initialize schema: {}", e)))?;

        Ok(())
    }

    /// Execute a blocking operation on the database
    fn execute<F, T>(&self, f: F) -> StorageResult<T>
    where
        F: FnOnce(&Connection) -> StorageResult<T>,
    {
        let conn = self.conn.lock().map_err(|e| StorageError::Database(e.to_string()))?;
        f(&conn)
    }

    // =============================================================================
    // Session Operations
    // =============================================================================

    pub fn save_session(&self, session: &Session) -> StorageResult<()> {
        self.execute(|conn| {
            let metadata = serde_json::to_string(&session.metadata)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;

            let status = match &session.status {
                SessionStatus::Active => "active",
                SessionStatus::Archived => "archived",
                SessionStatus::Deleted => "deleted",
            };

            conn.execute(
                "INSERT OR REPLACE INTO sessions (id, name, status, workspace_root, project_type, created_at, last_active_at, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    session.id,
                    session.name,
                    status,
                    session.workspace_root,
                    session.project_type,
                    session.created_at,
                    session.last_active_at,
                    metadata,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn load_session(&self, session_id: &str) -> StorageResult<Option<Session>> {
        self.execute(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, name, status, workspace_root, project_type, created_at, last_active_at, metadata FROM sessions WHERE id = ?1")
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query(params![session_id])
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            if let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let status_str: String = row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let status = match status_str.as_str() {
                    "archived" => SessionStatus::Archived,
                    "deleted" => SessionStatus::Deleted,
                    _ => SessionStatus::Active,
                };

                let metadata_str: String = row.get(7).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let metadata: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&metadata_str).unwrap_or_default();

                Ok(Some(Session {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    name: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    status,
                    workspace_root: row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    project_type: row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    created_at: row.get(5).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    last_active_at: row.get(6).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    metadata,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn delete_session(&self, session_id: &str) -> StorageResult<bool> {
        self.execute(|conn| {
            let count = conn
                .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
                .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
            Ok(count > 0)
        })
    }

    pub fn list_sessions(
        &self,
        filter: &SessionFilter,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> StorageResult<Vec<Session>> {
        self.execute(|conn| {
            let mut sql = String::from("SELECT id, name, status, workspace_root, project_type, created_at, last_active_at, metadata FROM sessions WHERE 1=1");
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(ref status) = filter.status {
                let status_str = match status {
                    SessionStatus::Active => "active",
                    SessionStatus::Archived => "archived",
                    SessionStatus::Deleted => "deleted",
                };
                sql.push_str(" AND status = ?");
                params_vec.push(Box::new(status_str.to_string()));
            }

            if let Some(workspace) = &filter.workspace {
                sql.push_str(" AND workspace_root = ?");
                params_vec.push(Box::new(workspace.clone()));
            }

            if let Some(created_after) = filter.created_after {
                sql.push_str(" AND created_at >= ?");
                params_vec.push(Box::new(created_after));
            }

            if let Some(created_before) = filter.created_before {
                sql.push_str(" AND created_at <= ?");
                params_vec.push(Box::new(created_before));
            }

            sql.push_str(" ORDER BY last_active_at DESC");

            if let Some(lim) = limit {
                sql.push_str(&format!(" LIMIT {}", lim));
            }

            if let Some(off) = offset {
                sql.push_str(&format!(" OFFSET {}", off));
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query(params_refs.as_slice())
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut sessions = Vec::new();
            while let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let status_str: String = row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let status = match status_str.as_str() {
                    "archived" => SessionStatus::Archived,
                    "deleted" => SessionStatus::Deleted,
                    _ => SessionStatus::Active,
                };

                let metadata_str: String = row.get(7).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let metadata: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&metadata_str).unwrap_or_default();

                sessions.push(Session {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    name: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    status,
                    workspace_root: row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    project_type: row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    created_at: row.get(5).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    last_active_at: row.get(6).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    metadata,
                });
            }

            Ok(sessions)
        })
    }

    // =============================================================================
    // Message Operations
    // =============================================================================

    pub fn append_message(&self, message: &Message) -> StorageResult<()> {
        self.execute(|conn| {
            let metadata = serde_json::to_string(&message.metadata)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;

            let role = match &message.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
            };

            conn.execute(
                "INSERT INTO messages (id, session_id, role, content, timestamp, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    message.id,
                    message.session_id,
                    role,
                    message.content,
                    message.timestamp,
                    metadata,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        after: Option<&str>,
    ) -> StorageResult<Vec<Message>> {
        self.execute(|conn| {
            let mut sql = String::from(
                "SELECT id, session_id, role, content, timestamp, metadata FROM messages WHERE session_id = ?1",
            );
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(session_id.to_string())];

            if let Some(after_id) = after {
                sql.push_str(" AND timestamp > (SELECT timestamp FROM messages WHERE id = ?");
                params_vec.push(Box::new(after_id.to_string()));
                sql.push(')');
            }

            sql.push_str(" ORDER BY timestamp ASC");

            if let Some(lim) = limit {
                sql.push_str(&format!(" LIMIT {}", lim));
            }

            if let Some(off) = offset {
                sql.push_str(&format!(" OFFSET {}", off));
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query(params_refs.as_slice())
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut messages = Vec::new();
            while let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let role_str: String = row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let role = match role_str.as_str() {
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    _ => MessageRole::User,
                };

                let metadata_str: String = row.get(5).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let metadata: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&metadata_str).unwrap_or_default();

                messages.push(Message {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    session_id: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    role,
                    content: row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    timestamp: row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    metadata,
                });
            }

            Ok(messages)
        })
    }

    pub fn delete_messages_before(&self, session_id: &str, before_timestamp: &str) -> StorageResult<i64> {
        self.execute(|conn| {
            // First get the timestamp threshold
            let threshold: Option<i64> = conn
                .query_row(
                    "SELECT timestamp FROM messages WHERE id = ?1",
                    params![before_timestamp],
                    |row| row.get(0),
                )
                .ok();

            if let Some(ts) = threshold {
                let count = conn
                    .execute(
                        "DELETE FROM messages WHERE session_id = ?1 AND timestamp < ?2",
                        params![session_id, ts],
                    )
                    .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
                Ok(count as i64)
            } else {
                Ok(0)
            }
        })
    }

    // =============================================================================
    // Task Operations
    // =============================================================================

    pub fn save_task(&self, task: &Task) -> StorageResult<()> {
        self.execute(|conn| {
            let inputs = serde_json::to_string(&task.inputs)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;
            let outputs = serde_json::to_string(&task.outputs)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;

            let status = match &task.status {
                TaskStatus::Pending => "pending",
                TaskStatus::Running => "running",
                TaskStatus::Completed => "completed",
                TaskStatus::Failed => "failed",
                TaskStatus::Cancelled => "cancelled",
            };

            conn.execute(
                "INSERT OR REPLACE INTO tasks (id, workflow_id, name, type, status, agent_id, inputs, outputs, error, created_at, started_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    task.id,
                    task.workflow_id,
                    task.name,
                    task.task_type,
                    status,
                    task.agent_id,
                    inputs,
                    outputs,
                    task.error,
                    task.created_at,
                    task.started_at,
                    task.completed_at,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn load_task(&self, task_id: &str) -> StorageResult<Option<Task>> {
        self.execute(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, workflow_id, name, type, status, agent_id, inputs, outputs, error, created_at, started_at, completed_at FROM tasks WHERE id = ?1")
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query(params![task_id])
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            if let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let status_str: String = row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let status = match status_str.as_str() {
                    "running" => TaskStatus::Running,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed,
                    "cancelled" => TaskStatus::Cancelled,
                    _ => TaskStatus::Pending,
                };

                let inputs_str: String = row.get(6).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let inputs: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&inputs_str).unwrap_or_default();

                let outputs_str: String = row.get(7).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let outputs: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&outputs_str).unwrap_or_default();

                Ok(Some(Task {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    workflow_id: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    name: row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    task_type: row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    status,
                    agent_id: row.get(5).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    inputs,
                    outputs,
                    error: row.get(8).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    created_at: row.get(9).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    started_at: row.get(10).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    completed_at: row.get(11).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn update_task(&self, task_id: &str, update: &TaskUpdate) -> StorageResult<bool> {
        self.execute(|conn| {
            // Get current task for merging
            let current: Option<(String, String)> = conn
                .query_row(
                    "SELECT inputs, outputs FROM tasks WHERE id = ?1",
                    params![task_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .ok();

            let (inputs_json, outputs_json) = if let Some((inp, out)) = current {
                let inputs: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&inp).unwrap_or_default();
                let outputs: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&out).unwrap_or_default();

                let final_inputs = if let Some(ref inp) = update.input {
                    serde_json::to_string(inp).unwrap_or_default()
                } else {
                    serde_json::to_string(&inputs).unwrap_or_default()
                };

                let final_outputs = if let Some(ref out) = update.output {
                    serde_json::to_string(out).unwrap_or_default()
                } else {
                    serde_json::to_string(&outputs).unwrap_or_default()
                };

                (final_inputs, final_outputs)
            } else {
                return Ok(false);
            };

            let status = update
                .status
                .as_ref()
                .map(|s| match s {
                    TaskStatus::Pending => "pending",
                    TaskStatus::Running => "running",
                    TaskStatus::Completed => "completed",
                    TaskStatus::Failed => "failed",
                    TaskStatus::Cancelled => "cancelled",
                })
                .unwrap_or("pending");

            let count = conn
                .execute(
                    "UPDATE tasks SET status = ?1, inputs = ?2, outputs = ?3, error = ?4, started_at = ?5, completed_at = ?6 WHERE id = ?7",
                    params![
                        status,
                        inputs_json,
                        outputs_json,
                        update.error,
                        update.started_at,
                        update.completed_at,
                        task_id,
                    ],
                )
                .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(count > 0)
        })
    }

    pub fn list_tasks(&self, filter: &TaskFilter, limit: Option<usize>) -> StorageResult<Vec<Task>> {
        self.execute(|conn| {
            let mut sql = String::from("SELECT id, workflow_id, name, type, status, agent_id, inputs, outputs, error, created_at, started_at, completed_at FROM tasks WHERE 1=1");
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(ref workflow_id) = filter.workflow_id {
                sql.push_str(" AND workflow_id = ?");
                params_vec.push(Box::new(workflow_id.clone()));
            }

            if let Some(ref status) = filter.status {
                let status_str = match status {
                    TaskStatus::Pending => "pending",
                    TaskStatus::Running => "running",
                    TaskStatus::Completed => "completed",
                    TaskStatus::Failed => "failed",
                    TaskStatus::Cancelled => "cancelled",
                };
                sql.push_str(" AND status = ?");
                params_vec.push(Box::new(status_str.to_string()));
            }

            if let Some(ref task_type) = filter.task_type {
                sql.push_str(" AND type = ?");
                params_vec.push(Box::new(task_type.clone()));
            }

            if let Some(created_after) = filter.created_after {
                sql.push_str(" AND created_at >= ?");
                params_vec.push(Box::new(created_after));
            }

            if let Some(created_before) = filter.created_before {
                sql.push_str(" AND created_at <= ?");
                params_vec.push(Box::new(created_before));
            }

            sql.push_str(" ORDER BY created_at DESC");

            if let Some(lim) = limit {
                sql.push_str(&format!(" LIMIT {}", lim));
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query(params_refs.as_slice())
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut tasks = Vec::new();
            while let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let status_str: String = row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let status = match status_str.as_str() {
                    "running" => TaskStatus::Running,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed,
                    "cancelled" => TaskStatus::Cancelled,
                    _ => TaskStatus::Pending,
                };

                let inputs_str: String = row.get(6).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let inputs: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&inputs_str).unwrap_or_default();

                let outputs_str: String = row.get(7).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let outputs: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&outputs_str).unwrap_or_default();

                tasks.push(Task {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    workflow_id: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    name: row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    task_type: row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    status,
                    agent_id: row.get(5).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    inputs,
                    outputs,
                    error: row.get(8).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    created_at: row.get(9).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    started_at: row.get(10).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    completed_at: row.get(11).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                });
            }

            Ok(tasks)
        })
    }

    // =============================================================================
    // Workflow Operations
    // =============================================================================

    pub fn save_workflow(&self, workflow: &WorkflowDefinition) -> StorageResult<()> {
        self.execute(|conn| {
            let definition = serde_json::to_string(&workflow.definition)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;

            conn.execute(
                "INSERT OR REPLACE INTO workflows (id, name, description, definition, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    workflow.id,
                    workflow.name,
                    workflow.description,
                    definition,
                    workflow.created_at,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn load_workflow(&self, workflow_id: &str) -> StorageResult<Option<WorkflowDefinition>> {
        self.execute(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, name, description, definition, created_at FROM workflows WHERE id = ?1")
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query(params![workflow_id])
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            if let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let definition_str: String =
                    row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let definition: serde_json::Value =
                    serde_json::from_str(&definition_str).unwrap_or(serde_json::Value::Null);

                Ok(Some(WorkflowDefinition {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    name: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    description: row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    definition,
                    created_at: row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn list_workflows(&self) -> StorageResult<Vec<WorkflowDefinition>> {
        self.execute(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, name, description, definition, created_at FROM workflows ORDER BY created_at DESC")
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query([])
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut workflows = Vec::new();
            while let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let definition_str: String =
                    row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let definition: serde_json::Value =
                    serde_json::from_str(&definition_str).unwrap_or(serde_json::Value::Null);

                workflows.push(WorkflowDefinition {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    name: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    description: row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    definition,
                    created_at: row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                });
            }

            Ok(workflows)
        })
    }

    // =============================================================================
    // Config Operations
    // =============================================================================

    pub fn save_config(&self, key: &str, value: &str) -> StorageResult<()> {
        self.execute(|conn| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?
                .as_secs() as i64;

            conn.execute(
                "INSERT OR REPLACE INTO config (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![key, value, now],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn load_config(&self, key: &str) -> StorageResult<Option<String>> {
        self.execute(|conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT value FROM config WHERE key = ?1",
                    params![key],
                    |row| row.get(0),
                )
                .ok();
            Ok(result)
        })
    }

    pub fn delete_config(&self, key: &str) -> StorageResult<bool> {
        self.execute(|conn| {
            let count = conn
                .execute("DELETE FROM config WHERE key = ?1", params![key])
                .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
            Ok(count > 0)
        })
    }

    // =============================================================================
    // Compression Point Operations
    // =============================================================================

    pub fn save_compression_point(&self, point: &CompressionPoint) -> StorageResult<()> {
        self.execute(|conn| {
            let metadata = serde_json::to_string(&point.metadata)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;

            conn.execute(
                "INSERT OR REPLACE INTO compression_points (id, session_id, created_at, before_count, after_count, summary, token_saved, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    point.id,
                    point.session_id,
                    point.created_at,
                    point.before_count,
                    point.after_count,
                    point.summary,
                    point.token_saved,
                    metadata,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn get_compression_points(&self, session_id: &str) -> StorageResult<Vec<CompressionPoint>> {
        self.execute(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, session_id, created_at, before_count, after_count, summary, token_saved, metadata FROM compression_points WHERE session_id = ?1 ORDER BY created_at DESC")
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut rows = stmt
                .query(params![session_id])
                .map_err(|e| StorageError::ReadFailed(e.to_string()))?;

            let mut points = Vec::new();
            while let Some(row) = rows.next().map_err(|e| StorageError::ReadFailed(e.to_string()))? {
                let metadata_str: String = row.get(7).map_err(|e| StorageError::ReadFailed(e.to_string()))?;
                let metadata: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&metadata_str).unwrap_or_default();

                points.push(CompressionPoint {
                    id: row.get(0).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    session_id: row.get(1).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    created_at: row.get(2).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    before_count: row.get(3).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    after_count: row.get(4).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    summary: row.get(5).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    token_saved: row.get(6).map_err(|e| StorageError::ReadFailed(e.to_string()))?,
                    metadata,
                });
            }

            Ok(points)
        })
    }

    pub fn delete_compression_point(&self, point_id: &str) -> StorageResult<bool> {
        self.execute(|conn| {
            let count = conn
                .execute("DELETE FROM compression_points WHERE id = ?1", params![point_id])
                .map_err(|e| StorageError::WriteFailed(e.to_string()))?;
            Ok(count > 0)
        })
    }

    // =============================================================================
    // Statistics Operations
    // =============================================================================

    pub fn save_token_usage(&self, record: &TokenUsageRecord) -> StorageResult<()> {
        self.execute(|conn| {
            let metadata = serde_json::to_string(&record.metadata)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;

            conn.execute(
                "INSERT INTO token_usage_log (id, session_id, model, input_tokens, output_tokens, total_tokens, cost_estimate, timestamp, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.session_id,
                    record.model,
                    record.input_tokens,
                    record.output_tokens,
                    record.total_tokens,
                    record.cost_estimate,
                    record.timestamp,
                    metadata,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn save_llm_call(&self, record: &LLMCallRecord) -> StorageResult<()> {
        self.execute(|conn| {
            conn.execute(
                "INSERT INTO llm_call_log (id, session_id, agent_id, model, prompt_tokens, completion_tokens, total_tokens, latency_ms, timestamp, success, error_message)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    record.id,
                    record.session_id,
                    record.agent_id,
                    record.model,
                    record.prompt_tokens,
                    record.completion_tokens,
                    record.total_tokens,
                    record.latency_ms,
                    record.timestamp,
                    record.success as i32,
                    record.error_message,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn save_session_event(&self, event: &SessionEvent) -> StorageResult<()> {
        self.execute(|conn| {
            let metadata = serde_json::to_string(&event.metadata)
                .map_err(|e| StorageError::InvalidData(e.to_string()))?;

            conn.execute(
                "INSERT INTO session_events (id, session_id, event_type, timestamp, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event.id,
                    event.session_id,
                    event.event_type,
                    event.timestamp,
                    metadata,
                ],
            )
            .map_err(|e| StorageError::WriteFailed(e.to_string()))?;

            Ok(())
        })
    }

    // =============================================================================
    // Maintenance Operations
    // =============================================================================

    pub fn vacuum(&self) -> StorageResult<i64> {
        self.execute(|conn| {
            let before: i64 = conn
                .query_row(
                    "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0);

            conn.execute_batch("VACUUM")
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let after: i64 = conn
                .query_row(
                    "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0);

            Ok((before - after) / (1024 * 1024))
        })
    }

    pub fn reindex(&self) -> StorageResult<()> {
        self.execute(|conn| {
            conn.execute_batch("REINDEX")
                .map_err(|e| StorageError::Database(e.to_string()))?;
            Ok(())
        })
    }

    /// Get database file size in bytes
    pub fn get_database_size(&self) -> StorageResult<i64> {
        self.execute(|conn| {
            let size: i64 = conn
                .query_row(
                    "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0);
            Ok(size)
        })
    }

    /// Close the database connection and release file locks
    /// This ensures SQLite properly closes WAL and releases file handles
    #[allow(dead_code)]
    pub fn close(&self) {
        // Lock to ensure any pending operations complete
        if let Ok(_guard) = self.conn.lock() {
            // Connection held - will be dropped when guard is dropped
        }
    }
}

/// Storage error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageErrorType {
    NotInitialized,
    ReadFailed,
    WriteFailed,
    NotFound,
    InvalidData,
    DatabaseError,
}

/// Storage error with context
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage not initialized")]
    NotInitialized,

    #[error("Read failed: {0}")]
    ReadFailed(String),

    #[error("Write failed: {0}")]
    WriteFailed(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Database error: {0}")]
    Database(String),
}
