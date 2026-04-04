//! Monitor Tests
//!
//! Unit tests for the monitor module.

use monitor::{
    MonitorImpl, TokenUsage, SystemStats,
};

#[tokio::test]
async fn test_monitor_initialization() {
    let monitor = MonitorImpl::new();
    assert!(!monitor.is_initialized());

    monitor.initialize().await.unwrap();
    assert!(monitor.is_initialized());
}

#[tokio::test]
async fn test_start_stop_monitoring() {
    let monitor = MonitorImpl::new();
    monitor.initialize().await.unwrap();

    assert!(!monitor.is_running());
    monitor.start_monitoring().await.unwrap();
    assert!(monitor.is_running());

    monitor.stop_monitoring().await.unwrap();
    assert!(!monitor.is_running());
}

#[tokio::test]
async fn test_record_token_usage() {
    let monitor = MonitorImpl::new();
    monitor.initialize().await.unwrap();

    monitor.record_token_usage(100, "claude", "input").await;
    monitor.record_token_usage(50, "claude", "output").await;

    let usage = monitor.get_token_usage(None, None, None).await.unwrap();
    assert_eq!(usage.total, 150);
}

#[tokio::test]
async fn test_get_stats() {
    let monitor = MonitorImpl::new();
    monitor.initialize().await.unwrap();

    let stats = monitor.get_stats(None, None).await.unwrap();
    assert_eq!(stats.tokens.total, 0);
}

#[tokio::test]
async fn test_get_status() {
    let monitor = MonitorImpl::new();
    monitor.initialize().await.unwrap();

    let status = monitor.get_status(None, None).await.unwrap();
    assert!(status.initialized);
    assert!(!status.running);
}

#[tokio::test]
async fn test_collect_metrics() {
    let monitor = MonitorImpl::new();
    monitor.initialize().await.unwrap();

    let metrics = monitor.collect_metrics().await.unwrap();
    assert_eq!(metrics.active_sessions, 0);
}

#[tokio::test]
async fn test_reset_stats() {
    let monitor = MonitorImpl::new();
    monitor.initialize().await.unwrap();

    monitor.record_token_usage(100, "claude", "input").await;
    monitor.reset_stats().await;

    let usage = monitor.get_token_usage(None, None, None).await.unwrap();
    assert_eq!(usage.total, 0);
}

#[tokio::test]
async fn test_get_summary() {
    let monitor = MonitorImpl::new();
    monitor.initialize().await.unwrap();

    let summary = monitor.get_summary().await;
    assert!(summary.contains("Sessions:"));
    assert!(summary.contains("Tokens:"));
}

#[tokio::test]
async fn test_token_usage_add() {
    let mut usage = TokenUsage::default();
    usage.add(100, "claude", "input");
    usage.add(50, "claude", "output");

    assert_eq!(usage.total, 150);
    assert_eq!(usage.by_model.get("claude"), Some(&150));
    assert_eq!(usage.by_type.get("input"), Some(&100));
}

#[tokio::test]
async fn test_system_stats_default() {
    let stats = SystemStats::default();
    assert_eq!(stats.tokens.total, 0);
    assert_eq!(stats.sessions.active_count, 0);
}
