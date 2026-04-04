//! Logging System Tests

use logging_system::{
    ExportFormat, LogEntry, LogFilter, LogLevel, LoggingSystem, LoggingSystemImpl,
};

#[tokio::test]
async fn test_logging_system_init() {
    let logging = LoggingSystemImpl::new().unwrap();
    assert!(!logging.is_initialized());

    logging.init().await.unwrap();
    assert!(logging.is_initialized());
}

#[tokio::test]
async fn test_logging_system_log() {
    let logging = LoggingSystemImpl::new().unwrap();
    logging.init().await.unwrap();

    let entry = LogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now(),
        level: LogLevel::Info,
        module: "test".to_string(),
        session_id: None,
        message: "Test message".to_string(),
        context: std::collections::HashMap::new(),
        error: None,
    };

    let result = logging.log(entry).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_log_level_filtering() {
    let logging = LoggingSystemImpl::new().unwrap();
    logging.init().await.unwrap();

    logging.set_level(LogLevel::Warn).await.unwrap();

    let entry = LogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now(),
        level: LogLevel::Debug,
        module: "test".to_string(),
        session_id: None,
        message: "Debug message".to_string(),
        context: std::collections::HashMap::new(),
        error: None,
    };

    logging.log(entry).await.unwrap();

    let filter = LogFilter {
        level: Some(LogLevel::Debug),
        module: None,
        session_id: None,
        since: None,
        until: None,
        message_pattern: None,
    };

    let logs = logging.get_logs(filter).await.unwrap();
    assert_eq!(logs.len(), 0);
}

#[tokio::test]
async fn test_get_stats() {
    let logging = LoggingSystemImpl::new().unwrap();
    logging.init().await.unwrap();

    let entry = LogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now(),
        level: LogLevel::Info,
        module: "test_module".to_string(),
        session_id: None,
        message: "Test message".to_string(),
        context: std::collections::HashMap::new(),
        error: None,
    };

    logging.log(entry).await.unwrap();

    let stats = logging.get_stats().await;
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.entries_by_module.get("test_module"), Some(&1));
}

#[tokio::test]
async fn test_log_level_parse() {
    assert_eq!(LogLevel::parse("debug").unwrap(), LogLevel::Debug);
    assert_eq!(LogLevel::parse("INFO").unwrap(), LogLevel::Info);
    assert_eq!(LogLevel::parse("warn").unwrap(), LogLevel::Warn);
    assert_eq!(LogLevel::parse("error").unwrap(), LogLevel::Error);
    assert!(LogLevel::parse("invalid").is_err());
}

#[tokio::test]
async fn test_export() {
    let logging = LoggingSystemImpl::new().unwrap();
    logging.init().await.unwrap();

    let entry = LogEntry {
        id: "test-id".to_string(),
        timestamp: std::time::SystemTime::now(),
        level: LogLevel::Info,
        module: "test".to_string(),
        session_id: None,
        message: "Test message".to_string(),
        context: std::collections::HashMap::new(),
        error: None,
    };

    logging.log(entry).await.unwrap();

    let filter = LogFilter::default();
    let json = logging.export(ExportFormat::Json, filter).await.unwrap();
    assert!(json.contains("test-id"));
    assert!(json.contains("Test message"));
}
