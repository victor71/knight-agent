//! Bootstrap module integration tests
//!
//! Tests for the 8-stage initialization system

use bootstrap::*;

#[tokio::test]
async fn test_bootstrap_stage_enum() {
    assert_eq!(BootstrapStage::Stage1Infrastructure.as_u8(), 1);
    assert_eq!(BootstrapStage::Stage1Infrastructure.name(), "Infrastructure");
    assert_eq!(
        BootstrapStage::Stage1Infrastructure.to_string(),
        "Stage 1: Infrastructure"
    );
}

#[tokio::test]
async fn test_bootstrap_stage_modules() {
    let stage1 = BootstrapStage::Stage1Infrastructure;
    assert_eq!(stage1.modules(), vec!["logging-system"]);

    let stage8 = BootstrapStage::Stage8SecurityLayer;
    assert_eq!(stage8.modules(), vec!["sandbox", "ipc-contract"]);
}

#[tokio::test]
async fn test_system_new() {
    let system = KnightAgentSystem::new();
    assert!(!system.is_initialized().await);
    assert_eq!(system.stage().await, BootstrapStage::Stage1Infrastructure);
}

#[tokio::test]
async fn test_system_bootstrap() {
    let system = KnightAgentSystem::new();
    system.bootstrap().await.unwrap();
    assert!(system.is_initialized().await);
    assert_eq!(system.stage().await, BootstrapStage::Stage8SecurityLayer);
}

#[tokio::test]
async fn test_system_status() {
    let system = KnightAgentSystem::new();
    system.bootstrap().await.unwrap();

    let status = system.status().await;
    assert!(status.initialized);
    assert!(status.ready);
    assert_eq!(status.stage, 8);
    assert_eq!(status.module_count, 23);
    assert_eq!(status.initialized_count, 23);
}

#[tokio::test]
async fn test_module_statuses() {
    let system = KnightAgentSystem::new();
    system.bootstrap().await.unwrap();

    let modules = system.module_statuses().await;
    assert_eq!(modules.len(), 23);

    // Check that logging-system is initialized
    let logging_status = system.module_status("logging-system").await;
    assert!(logging_status.is_some());
    assert!(logging_status.unwrap().initialized);
}

#[tokio::test]
async fn test_health_check() {
    let system = KnightAgentSystem::new();
    system.bootstrap().await.unwrap();

    let health = system.health_check(false).await.unwrap();
    assert!(health.healthy);

    let detailed_health = system.health_check(true).await.unwrap();
    assert!(detailed_health.healthy);
    assert_eq!(detailed_health.details.len(), 23);
}

#[tokio::test]
async fn test_system_stop() {
    let system = KnightAgentSystem::new();
    system.bootstrap().await.unwrap();

    let stopped = system.stop(true, 5000).await.unwrap();
    assert!(stopped);
    assert!(!system.is_initialized().await);
}

#[tokio::test]
async fn test_system_restart() {
    let system = KnightAgentSystem::new();
    system.bootstrap().await.unwrap();

    let restarted = system.restart(true).await.unwrap();
    assert!(restarted);
    assert!(system.is_initialized().await);
}

#[tokio::test]
async fn test_bootstrap_config_default() {
    let config = BootstrapConfig::default();
    assert_eq!(config.workspace, ".");
    assert!(!config.parallel_init);
    assert_eq!(config.init_timeout_ms, 60000);
    assert!(config.retry_on_failure);
    assert_eq!(config.max_retries, 3);
}

#[tokio::test]
async fn test_module_status() {
    let status = ModuleStatus::new("test-module".to_string(), BootstrapStage::Stage1Infrastructure);
    assert!(!status.initialized);
    assert!(!status.healthy);
    assert_eq!(status.stage, 1);

    let initialized = status.clone().initialized();
    assert!(initialized.initialized);
    assert!(!initialized.healthy);

    let healthy = initialized.healthy();
    assert!(healthy.initialized);
    assert!(healthy.healthy);
}

#[tokio::test]
async fn test_already_initialized_error() {
    let system = KnightAgentSystem::new();
    system.bootstrap().await.unwrap();

    let result = system.bootstrap().await;
    assert!(matches!(result, Err(BootstrapError::AlreadyInitialized)));
}

#[tokio::test]
async fn test_version_info() {
    let system = KnightAgentSystem::new();
    let version = system.version();
    assert!(!version.version.is_empty());
}
