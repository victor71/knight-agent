//! Storage Service Tests
//!
//! Unit tests for the storage_service module.

use storage_service::{
    CompressionPoint, Database, LLMCallRecord, Message, MessageRole, Session,
    SessionEvent, SessionFilter, SessionStatus, StorageConfig, StorageService, StorageServiceImpl,
    Task, TaskFilter, TaskStatus, TaskUpdate, TokenUsageRecord, WorkflowDefinition,
};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

/// RAII guard for automatic cleanup of test database files
struct TempStorage {
    storage: StorageServiceImpl,
    path: String,
}

impl TempStorage {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        let path = format!("./test_storage_{}_{}.db", timestamp, id);
        let config = StorageConfig {
            database_path: path.clone(),
            ..Default::default()
        };
        let storage = StorageServiceImpl::with_config(config).unwrap();

        Self { storage, path }
    }
}

impl Drop for TempStorage {
    fn drop(&mut self) {
        // Wait for database connection to be released
        std::thread::sleep(std::time::Duration::from_millis(50));
        // Clean up database file
        if let Err(e) = std::fs::remove_file(&self.path) {
            eprintln!("Failed to remove {}: {}", self.path, e);
        }
        let wal_path = format!("{}-wal", &self.path);
        let shm_path = format!("{}-shm", &self.path);
        let _ = std::fs::remove_file(&wal_path);
        let _ = std::fs::remove_file(&shm_path);
    }
}

fn temp_db_path() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    format!("./test_temp_{}_{}.db", timestamp, id)
}

/// RAII guard for automatic cleanup of test database files
struct TempDatabase {
    db: Database,
    path: String,
}

impl TempDatabase {
    fn new() -> Self {
        let path = temp_db_path();
        let db = Database::open(std::path::Path::new(&path)).expect("should open database");
        Self { db, path }
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        // Try to close database and clean up files
        let path = self.path.clone();
        self.db.close();
        // Wait for file handles to be released (especially on Windows)
        std::thread::sleep(std::time::Duration::from_millis(100));
        // Clean up database file
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", &path));
        let _ = std::fs::remove_file(format!("{}-shm", &path));
    }
}

fn create_test_session(id: &str, name: &str) -> Session {
    Session {
        id: id.to_string(),
        name: name.to_string(),
        status: SessionStatus::Active,
        workspace_root: "/test/workspace".to_string(),
        project_type: Some("rust".to_string()),
        created_at: 1000,
        last_active_at: 1000,
        metadata: HashMap::new(),
    }
}

fn create_test_message(id: &str, session_id: &str, content: &str) -> Message {
    Message {
        id: id.to_string(),
        session_id: session_id.to_string(),
        role: MessageRole::User,
        content: content.to_string(),
        timestamp: 1000,
        metadata: HashMap::new(),
    }
}

fn create_test_task(id: &str, name: &str, task_type: &str) -> Task {
    Task {
        id: id.to_string(),
        workflow_id: None,
        name: name.to_string(),
        task_type: task_type.to_string(),
        status: TaskStatus::Pending,
        agent_id: None,
        inputs: HashMap::new(),
        outputs: HashMap::new(),
        error: None,
        created_at: 1000,
        started_at: None,
        completed_at: None,
    }
}

// Helper to create an isolated storage service for async tests
fn create_test_storage() -> TempStorage {
    TempStorage::new()
}

// =============================================================================
// Database Tests
// =============================================================================

#[test]
fn test_database_open_and_init() {
    let _temp_db = TempDatabase::new();
    // Database is opened and will be cleaned up when _temp_db goes out of scope
}

#[test]
fn test_database_session_crud() {
    let temp_db = TempDatabase::new();
    let db = &temp_db.db;

    // Save session
    let session = create_test_session("s1", "Test Session");
    db.save_session(&session).expect("should save session");

    // Load session
    let loaded = db.load_session("s1").expect("should load session");
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, "s1");
    assert_eq!(loaded.name, "Test Session");
    assert_eq!(loaded.status, SessionStatus::Active);

    // Delete session
    let deleted = db.delete_session("s1").expect("should delete session");
    assert!(deleted);

    // Load deleted session
    let loaded = db.load_session("s1").expect("should load session");
    assert!(loaded.is_none());
}

#[test]
fn test_database_list_sessions() {
    let temp_db = TempDatabase::new();
    let db = &temp_db.db;

    // Save multiple sessions
    for i in 0..5 {
        let session = create_test_session(&format!("s{}", i), &format!("Session {}", i));
        db.save_session(&session).expect("should save session");
    }

    // List all
    let sessions = db.list_sessions(&SessionFilter::default(), None, None).expect("should list sessions");
    assert_eq!(sessions.len(), 5);

    // List with limit
    let sessions = db.list_sessions(&SessionFilter::default(), Some(2), None).expect("should list sessions");
    assert_eq!(sessions.len(), 2);
}

#[test]
fn test_database_message_crud() {
    let temp_db = TempDatabase::new();
    let db = &temp_db.db;

    // Create session first
    let session = create_test_session("s1", "Test");
    db.save_session(&session).expect("should save session");

    // Append message
    let msg = create_test_message("m1", "s1", "Hello");
    db.append_message(&msg).expect("should append message");

    // Get messages
    let messages = db.get_messages("s1", None, None, None).expect("should get messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello");
}

#[test]
fn test_database_task_crud() {
    let temp_db = TempDatabase::new();
    let db = &temp_db.db;

    // Save task
    let task = create_test_task("t1", "Test Task", "test");
    db.save_task(&task).expect("should save task");

    // Load task
    let loaded = db.load_task("t1").expect("should load task");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().name, "Test Task");

    // List tasks
    let tasks = db.list_tasks(&TaskFilter::default(), None).expect("should list tasks");
    assert_eq!(tasks.len(), 1);
}

#[test]
fn test_database_workflow_crud() {
    let temp_db = TempDatabase::new();
    let db = &temp_db.db;

    // Save workflow
    let workflow = WorkflowDefinition {
        id: "w1".to_string(),
        name: "Test Workflow".to_string(),
        description: Some("A test workflow".to_string()),
        definition: serde_json::json!({"steps": []}),
        created_at: 1000,
    };
    db.save_workflow(&workflow).expect("should save workflow");

    // Load workflow
    let loaded = db.load_workflow("w1").expect("should load workflow");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().name, "Test Workflow");

    // List workflows
    let workflows = db.list_workflows().expect("should list workflows");
    assert_eq!(workflows.len(), 1);
}

#[test]
fn test_database_config_crud() {
    let temp_db = TempDatabase::new();
    let db = &temp_db.db;

    // Save config
    db.save_config("key1", "value1").expect("should save config");

    // Load config
    let loaded = db.load_config("key1").expect("should load config");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), "value1");

    // Delete config
    let deleted = db.delete_config("key1").expect("should delete config");
    assert!(deleted);
}

#[test]
fn test_database_compression_point_crud() {
    let temp_db = TempDatabase::new();
    let db = &temp_db.db;

    // Create session first
    let session = create_test_session("s1", "Test");
    db.save_session(&session).expect("should save session");

    // Save compression point
    let point = CompressionPoint {
        id: "cp1".to_string(),
        session_id: "s1".to_string(),
        created_at: 1000,
        before_count: 100,
        after_count: 50,
        summary: "Compressed summary".to_string(),
        token_saved: 500,
        metadata: HashMap::new(),
    };
    db.save_compression_point(&point).expect("should save compression point");

    // Get compression points
    let points = db.get_compression_points("s1").expect("should get compression points");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].token_saved, 500);

    // Delete compression point
    let deleted = db.delete_compression_point("cp1").expect("should delete");
    assert!(deleted);
}

// =============================================================================
// StorageServiceImpl Tests
// =============================================================================

#[tokio::test]
async fn test_storage_service_init() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    assert!(!storage.is_initialized());

    storage.init().await.expect("should initialize");
    assert!(storage.is_initialized());
}

#[tokio::test]
async fn test_storage_service_session_operations() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    let session = create_test_session("s1", "Test Session");
    let result = storage.save_session(session).await;
    assert!(result.is_ok());

    let loaded = storage.load_session("s1").await.expect("should load session");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().name, "Test Session");
}

#[tokio::test]
async fn test_storage_service_message_operations() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    // Create session first
    let session = create_test_session("s1", "Test");
    storage.save_session(session).await.expect("should save session");

    // Append message
    let msg = create_test_message("m1", "s1", "Hello World");
    let result = storage.append_message(msg).await;
    assert!(result.is_ok());

    // Get messages
    let messages = storage.get_messages("s1", None, None, None).await.expect("should get messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello World");
}

#[tokio::test]
async fn test_storage_service_task_operations() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    let task = create_test_task("t1", "Test Task", "test");
    let result = storage.save_task(task).await;
    assert!(result.is_ok());

    let loaded = storage.load_task("t1").await.expect("should load task");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().name, "Test Task");

    // Update task
    let update = TaskUpdate {
        status: Some(TaskStatus::Completed),
        input: None,
        output: Some(HashMap::from([
            ("result".to_string(), serde_json::json!("success"))
        ])),
        error: None,
        started_at: Some(1000),
        completed_at: Some(2000),
    };

    let updated = storage.update_task("t1", update).await.expect("should update task");
    assert!(updated);

    let loaded = storage.load_task("t1").await.expect("should load updated task");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().status, TaskStatus::Completed);
}

#[tokio::test]
async fn test_storage_service_workflow_operations() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    let workflow = WorkflowDefinition {
        id: "w1".to_string(),
        name: "Test Workflow".to_string(),
        description: Some("A test".to_string()),
        definition: serde_json::json!({"steps": ["step1", "step2"]}),
        created_at: 1000,
    };

    let result = storage.save_workflow(workflow).await;
    assert!(result.is_ok());

    let loaded = storage.load_workflow("w1").await.expect("should load workflow");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().name, "Test Workflow");

    let workflows = storage.list_workflows().await.expect("should list workflows");
    assert_eq!(workflows.len(), 1);
}

#[tokio::test]
async fn test_storage_service_config_operations() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    // Save config
    let result = storage.save_config("test_key", serde_json::json!("test_value")).await;
    assert!(result.is_ok());

    // Load config
    let loaded = storage.load_config("test_key").await.expect("should load config");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), serde_json::json!("test_value"));

    // Delete config
    let deleted = storage.delete_config("test_key").await.expect("should delete config");
    assert!(deleted);
}

#[tokio::test]
async fn test_storage_service_compression_point_operations() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    // Create session first
    let session = create_test_session("s1", "Test");
    storage.save_session(session).await.expect("should save session");

    // Save compression point
    let point = CompressionPoint {
        id: "cp1".to_string(),
        session_id: "s1".to_string(),
        created_at: 1000,
        before_count: 100,
        after_count: 50,
        summary: "Compressed".to_string(),
        token_saved: 500,
        metadata: HashMap::new(),
    };

    let result = storage.save_compression_point(point).await;
    assert!(result.is_ok());

    // Get compression points
    let points = storage.get_compression_points("s1").await.expect("should get points");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].token_saved, 500);

    // Delete compression point
    let deleted = storage.delete_compression_point("cp1").await.expect("should delete");
    assert!(deleted);
}

#[tokio::test]
async fn test_storage_service_token_usage() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    let usage = TokenUsageRecord {
        id: "tu1".to_string(),
        session_id: "s1".to_string(),
        model: "claude-3".to_string(),
        input_tokens: 100,
        output_tokens: 200,
        total_tokens: 300,
        cost_estimate: 0.01,
        timestamp: 1000,
        metadata: HashMap::new(),
    };

    let result = storage.save_token_usage(usage).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_storage_service_llm_call() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    let call = LLMCallRecord {
        id: "llm1".to_string(),
        session_id: "s1".to_string(),
        agent_id: Some("a1".to_string()),
        model: "claude-3".to_string(),
        prompt_tokens: 100,
        completion_tokens: 200,
        total_tokens: 300,
        latency_ms: Some(150),
        timestamp: 1000,
        success: true,
        error_message: None,
    };

    let result = storage.save_llm_call(call).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_storage_service_session_event() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    let event = SessionEvent {
        id: "ev1".to_string(),
        session_id: "s1".to_string(),
        event_type: "created".to_string(),
        timestamp: 1000,
        metadata: HashMap::new(),
    };

    let result = storage.save_session_event(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_storage_service_stats() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    let stats = storage.get_stats().await.expect("should get stats");
    assert!(stats.database_size_mb >= 0.0);
}

#[tokio::test]
async fn test_storage_service_list_sessions_with_filter() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    // Create sessions with different statuses
    let active = create_test_session("s1", "Active Session");
    storage.save_session(active).await.expect("should save");

    let mut archived = create_test_session("s2", "Archived Session");
    archived.status = SessionStatus::Archived;
    storage.save_session(archived).await.expect("should save");

    // List all
    let all = storage.list_sessions(SessionFilter::default(), None, None).await.expect("should list");
    assert_eq!(all.len(), 2);

    // Filter by status
    let filter = SessionFilter {
        status: Some(SessionStatus::Active),
        ..Default::default()
    };
    let active_only = storage.list_sessions(filter, None, None).await.expect("should filter");
    assert_eq!(active_only.len(), 1);
    assert_eq!(active_only[0].status, SessionStatus::Active);
}

#[tokio::test]
async fn test_storage_service_delete_messages() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    // Create session and messages with different timestamps
    let session = create_test_session("s1", "Test");
    storage.save_session(session).await.expect("should save session");

    // Create messages with sequential timestamps
    let msg1 = Message {
        id: "m1".to_string(),
        session_id: "s1".to_string(),
        role: MessageRole::User,
        content: "First".to_string(),
        timestamp: 1000,
        metadata: HashMap::new(),
    };
    let msg2 = Message {
        id: "m2".to_string(),
        session_id: "s1".to_string(),
        role: MessageRole::User,
        content: "Second".to_string(),
        timestamp: 2000,
        metadata: HashMap::new(),
    };
    let msg3 = Message {
        id: "m3".to_string(),
        session_id: "s1".to_string(),
        role: MessageRole::User,
        content: "Third".to_string(),
        timestamp: 3000,
        metadata: HashMap::new(),
    };

    storage.append_message(msg1).await.expect("should append");
    storage.append_message(msg2).await.expect("should append");
    storage.append_message(msg3).await.expect("should append");

    // Delete messages before m3 (timestamp < 3000)
    let deleted = storage.delete_messages("s1", "m3").await.expect("should delete");
    assert_eq!(deleted, 2); // m1 and m2 should be deleted

    // Verify only m3 remains
    let messages = storage.get_messages("s1", None, None, None).await.expect("should get messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "m3");
}

#[tokio::test]
async fn test_storage_service_backup_restore() {
    let temp_storage = create_test_storage();
    let storage = &temp_storage.storage;
    storage.init().await.expect("should initialize");

    // Create a session
    let session = create_test_session("s1", "Test");
    storage.save_session(session).await.expect("should save");

    // Backup
    let backup_path = format!("{}.backup", temp_db_path());
    let result = storage.backup(&backup_path).await;
    assert!(result.is_ok());

    // Verify backup file exists
    assert!(std::path::Path::new(&backup_path).exists());

    // Clean up
    std::fs::remove_file(&backup_path).ok();
}
