//! Session Manager Tests
//!
//! Unit tests for the session manager module.

use session_manager::{
    CompressionMethod, CreateSessionRequest, Message, PathAction, ProjectType, SessionManagerImpl,
    SessionStatus,
};

fn create_test_request() -> CreateSessionRequest {
    CreateSessionRequest::new("/tmp/test")
        .name("test-session")
        .project_type(ProjectType::Rust)
}

#[tokio::test]
async fn test_create_session() {
    let manager = SessionManagerImpl::new();
    let request = create_test_request();

    let session = manager.create_session(request).await.unwrap();
    assert_eq!(session.metadata.name, "test-session");
    assert_eq!(session.metadata.workspace, "/tmp/test");
    assert_eq!(session.status, SessionStatus::Active);
}

#[tokio::test]
async fn test_get_session() {
    let manager = SessionManagerImpl::new();
    let request = create_test_request();
    let created = manager.create_session(request).await.unwrap();

    let retrieved = manager.get_session(&created.id).await.unwrap();
    assert_eq!(retrieved.id, created.id);
}

#[tokio::test]
async fn test_get_session_not_found() {
    let manager = SessionManagerImpl::new();
    let result = manager.get_session("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_sessions() {
    let manager = SessionManagerImpl::new();
    manager.create_session(create_test_request()).await.unwrap();
    manager
        .create_session(CreateSessionRequest::new("/tmp/test2"))
        .await
        .unwrap();

    let all = manager.list_sessions(None).await;
    assert_eq!(all.len(), 2);

    let active = manager.list_sessions(Some(SessionStatus::Active)).await;
    assert_eq!(active.len(), 2);
}

#[tokio::test]
async fn test_delete_session() {
    let manager = SessionManagerImpl::new();
    let request = create_test_request();
    let session = manager.create_session(request).await.unwrap();

    manager.delete_session(&session.id, false).await.unwrap();
    assert!(manager.get_session(&session.id).await.is_err());
}

#[tokio::test]
async fn test_archive_restore_session() {
    let manager = SessionManagerImpl::new();
    let request = create_test_request();
    let session = manager.create_session(request).await.unwrap();

    manager.archive_session(&session.id).await.unwrap();

    let archived = manager.get_session(&session.id).await.unwrap();
    assert_eq!(archived.status, SessionStatus::Archived);

    manager.restore_session(&session.id).await.unwrap();

    let restored = manager.get_session(&session.id).await.unwrap();
    assert_eq!(restored.status, SessionStatus::Active);
}

#[tokio::test]
async fn test_use_session() {
    let manager = SessionManagerImpl::new();
    let session1 = manager.create_session(create_test_request()).await.unwrap();
    let session2 = manager
        .create_session(CreateSessionRequest::new("/tmp/other"))
        .await
        .unwrap();

    manager.use_session(&session1.id).await.unwrap();
    let current = manager.get_current_session().await.unwrap();
    assert_eq!(current.id, session1.id);

    manager.use_session(&session2.id).await.unwrap();
    let current = manager.get_current_session().await.unwrap();
    assert_eq!(current.id, session2.id);
}

#[tokio::test]
async fn test_add_message() {
    let manager = SessionManagerImpl::new();
    let request = create_test_request();
    let session = manager.create_session(request).await.unwrap();

    let msg = Message::user("m1", "Hello world");
    let should_compress = manager.add_message(&session.id, msg).await.unwrap();

    assert!(!should_compress);
    assert_eq!(
        manager.get_stats(&session.id).await.unwrap().total_messages,
        1
    );
}

#[tokio::test]
async fn test_compress_context() {
    let manager = SessionManagerImpl::new();
    let request = create_test_request();
    let session = manager.create_session(request).await.unwrap();

    // Add some messages first
    for i in 0..5 {
        let msg = Message::user(format!("m{}", i), format!("Message {}", i));
        manager.add_message(&session.id, msg).await.unwrap();
    }

    let point = manager
        .compress_context(&session.id, CompressionMethod::Summary)
        .await
        .unwrap();

    assert_eq!(point.original_count, 5);
    assert_eq!(
        manager
            .get_stats(&session.id)
            .await
            .unwrap()
            .compression_count,
        1
    );
}

#[tokio::test]
async fn test_search_history() {
    let manager = SessionManagerImpl::new();
    let request = create_test_request();
    let session = manager.create_session(request).await.unwrap();

    manager
        .add_message(&session.id, Message::user("m1", "Hello world"))
        .await
        .unwrap();
    manager
        .add_message(&session.id, Message::assistant("m2", "Hi there!"))
        .await
        .unwrap();

    let results = manager
        .search_history("hello", Some(&session.id), 10)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].content.contains("Hello"));
}

#[tokio::test]
async fn test_check_path_access() {
    let manager = SessionManagerImpl::new();
    let request = CreateSessionRequest::new("/workspace/project");
    let session = manager.create_session(request).await.unwrap();

    let result = manager
        .check_path_access(
            &session.id,
            "/workspace/project/src/main.rs",
            PathAction::Read,
        )
        .await
        .unwrap();
    assert!(result.allowed);

    let result = manager
        .check_path_access(&session.id, "/etc/passwd", PathAction::Read)
        .await
        .unwrap();
    assert!(!result.allowed);
}

#[tokio::test]
async fn test_clear() {
    let manager = SessionManagerImpl::new();
    manager.create_session(create_test_request()).await.unwrap();
    manager
        .create_session(CreateSessionRequest::new("/tmp/other"))
        .await
        .unwrap();

    assert_eq!(manager.len().await, 2);
    manager.clear().await;
    assert!(manager.is_empty().await);
}

#[tokio::test]
async fn test_validate_path() {
    let manager = SessionManagerImpl::new();
    let request = CreateSessionRequest::new("/workspace/project");
    let session = manager.create_session(request).await.unwrap();

    let valid = manager
        .validate_path(&session.id, "/workspace/project/src")
        .await
        .unwrap();
    assert!(valid);

    let valid = manager
        .validate_path(&session.id, "/etc/passwd")
        .await
        .unwrap();
    assert!(!valid);
}
