//! Permission Service tests

use permission_service::{Permission, PermissionService, PermissionServiceImpl};

#[tokio::test]
async fn test_permission_service_new() {
    let service = PermissionServiceImpl::new().unwrap();
    assert_eq!(service.name(), "permission-service");
    assert!(!service.is_initialized());
}

#[tokio::test]
async fn test_initialize() {
    let service = PermissionServiceImpl::new().unwrap();
    service.initialize().await.unwrap();
    assert!(service.is_initialized());
}

#[tokio::test]
async fn test_grant_permission() {
    let service = PermissionServiceImpl::new().unwrap();
    service.initialize().await.unwrap();

    let permission = Permission::new("user-1".to_string(), "file:/test".to_string(), "read".to_string());
    service.grant_permission(permission.clone()).await.unwrap();

    assert!(service.check_permission(permission).await.unwrap());
}

#[tokio::test]
async fn test_revoke_permission() {
    let service = PermissionServiceImpl::new().unwrap();
    service.initialize().await.unwrap();

    let permission = Permission::new("user-1".to_string(), "file:/test".to_string(), "read".to_string());
    service.grant_permission(permission.clone()).await.unwrap();
    service.revoke_permission(permission.clone()).await.unwrap();

    assert!(!service.check_permission(permission).await.unwrap());
}

#[tokio::test]
async fn test_not_initialized() {
    let service = PermissionServiceImpl::new().unwrap();
    let permission = Permission::new("user-1".to_string(), "file:/test".to_string(), "read".to_string());

    let result = service.check_permission(permission).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_permission_new() {
    let permission = Permission::new("user-1".to_string(), "file:/test".to_string(), "read".to_string());
    assert_eq!(permission.user_id, "user-1");
    assert_eq!(permission.resource, "file:/test");
    assert_eq!(permission.action, "read");
}
