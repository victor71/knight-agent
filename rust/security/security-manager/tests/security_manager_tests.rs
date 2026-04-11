//! Security Manager Tests
//!
//! Unit tests for the security_manager module.

use security_manager::{
    AuditLogger, DefaultPolicy, EventResult, LogQuery, PolicyEffect, PolicyEngine, PolicyType,
    Principal, SecretManager, SecretPolicyConfig, SecurityConfig, SecurityContext, SecurityEvent,
    SecurityEventType, SecurityManager, SecurityManagerImpl, SecurityPolicy,
};

use security_manager::PermissionGrant;
use security_manager::PolicyRule;
use std::collections::HashMap;

// =============================================================================
// SecurityManagerImpl Tests
// =============================================================================

#[tokio::test]
async fn test_security_manager_init() {
    let security = SecurityManagerImpl::new().unwrap();
    assert!(!security.is_initialized());

    security.init().await.unwrap();
    assert!(security.is_initialized());
}

#[tokio::test]
async fn test_grant_permission() {
    let security = SecurityManagerImpl::new().unwrap();
    security.init().await.unwrap();

    let grant = PermissionGrant {
        principal: Principal::User("test-user".to_string()),
        resource: "file:/project/**".to_string(),
        actions: vec!["read".to_string(), "write".to_string()],
        conditions: vec![],
        expires_at: None,
    };

    let result = security.grant_permission(grant).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_check_permission() {
    let security = SecurityManagerImpl::new().unwrap();
    security.init().await.unwrap();

    // Grant permission
    let grant = PermissionGrant {
        principal: Principal::User("admin".to_string()),
        resource: "file:/project/**".to_string(),
        actions: vec!["read".to_string()],
        conditions: vec![],
        expires_at: None,
    };
    security.grant_permission(grant).await.unwrap();

    // Check permission
    let (allowed, reason) = security
        .check_permission("user:admin", "file:/project/test.txt", "read", None)
        .await
        .unwrap();

    assert!(allowed);
    assert!(reason.is_some());
}

#[tokio::test]
async fn test_default_deny() {
    let security = SecurityManagerImpl::new().unwrap();
    security.init().await.unwrap();

    // Check permission without grant - should be denied
    let (allowed, _) = security
        .check_permission("user:unknown", "file:/secret.txt", "read", None)
        .await
        .unwrap();

    assert!(!allowed);
}

#[tokio::test]
async fn test_policy_creation() {
    let security = SecurityManagerImpl::new().unwrap();
    security.init().await.unwrap();

    let policy = SecurityPolicy {
        id: "test-policy".to_string(),
        name: "Test Policy".to_string(),
        description: "Test".to_string(),
        policy_type: PolicyType::Rbac,
        enabled: true,
        rules: vec![PolicyRule {
            name: "allow-all".to_string(),
            effect: PolicyEffect::Allow,
            principal: Some("user:*".to_string()),
            resource: Some("*".to_string()),
            action: Some("read".to_string()),
            conditions: vec![],
        }],
        priority: 10,
    };

    let result = security.create_policy(policy).await;
    assert!(result.is_ok());

    let policies = security.list_policies(None).await.unwrap();
    assert_eq!(policies.len(), 1);
}

#[tokio::test]
async fn test_audit_logging() {
    let security = SecurityManagerImpl::new().unwrap();
    security.init().await.unwrap();

    let event = SecurityEvent {
        id: "event-1".to_string(),
        timestamp: std::time::SystemTime::now(),
        event_type: SecurityEventType::AccessRequest,
        principal: Principal::User("test-user".to_string()),
        resource: Some("resource:1".to_string()),
        action: Some("read".to_string()),
        result: EventResult::Allowed,
        reason: None,
        details: HashMap::new(),
    };

    let event_id = security.log_event(event).await.unwrap();
    assert_eq!(event_id, "event-1");

    let events = security.query_logs(LogQuery::default()).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_secret_management() {
    let security = SecurityManagerImpl::new().unwrap();
    security.init().await.unwrap();

    let result = security
        .store_secret("api-key", "supersecret123!@#", None)
        .await;
    assert!(result.is_ok());

    let value = security.get_secret("api-key").await.unwrap();
    assert_eq!(value, Some("supersecret123!@#".to_string()));

    let secrets = security.list_secrets().await.unwrap();
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].key, "api-key");

    let deleted = security.delete_secret("api-key").await.unwrap();
    assert!(deleted);
}

#[tokio::test]
async fn test_security_config() {
    let security = SecurityManagerImpl::new().unwrap();
    security.init().await.unwrap();

    let config = security.get_security_config().await.unwrap();
    assert_eq!(config.default_policy, DefaultPolicy::Deny);

    let new_config = SecurityConfig {
        default_policy: DefaultPolicy::Allow,
        ..Default::default()
    };

    let result = security.update_security_config(new_config).await;
    assert!(result.is_ok());

    let updated = security.get_security_config().await.unwrap();
    assert_eq!(updated.default_policy, DefaultPolicy::Allow);
}

// =============================================================================
// PolicyEngine Tests
// =============================================================================

#[tokio::test]
async fn test_policy_evaluation() {
    let engine = PolicyEngine::with_default();

    let policy = SecurityPolicy {
        id: "test-policy".to_string(),
        name: "Test Policy".to_string(),
        description: "Test".to_string(),
        policy_type: PolicyType::Rbac,
        enabled: true,
        rules: vec![
            PolicyRule {
                name: "allow-admin".to_string(),
                effect: PolicyEffect::Allow,
                principal: Some("user:admin".to_string()),
                resource: Some("*".to_string()),
                action: Some("*".to_string()),
                conditions: vec![],
            },
            PolicyRule {
                name: "deny-all".to_string(),
                effect: PolicyEffect::Deny,
                principal: Some("*".to_string()),
                resource: Some("*".to_string()),
                action: Some("*".to_string()),
                conditions: vec![],
            },
        ],
        priority: 10,
    };

    engine.add_policy(policy).await;

    let context = SecurityContext {
        principal: Principal::User("admin".to_string()),
        ..Default::default()
    };

    let result = engine.evaluate("test-policy", &context).await.unwrap();
    assert!(result.allowed);
    assert_eq!(result.matched_rule, Some("allow-admin".to_string()));
}

#[tokio::test]
async fn test_policy_default_deny() {
    let engine = PolicyEngine::new(DefaultPolicy::Deny);

    let policy = SecurityPolicy {
        id: "test-policy".to_string(),
        name: "Test Policy".to_string(),
        description: "Test".to_string(),
        policy_type: PolicyType::Rbac,
        enabled: true,
        rules: vec![],
        priority: 10,
    };

    engine.add_policy(policy).await;

    let context = SecurityContext {
        principal: Principal::User("anyone".to_string()),
        ..Default::default()
    };

    let result = engine.evaluate("test-policy", &context).await.unwrap();
    assert!(!result.allowed);
}

// =============================================================================
// SecretManager Tests
// =============================================================================

#[tokio::test]
async fn test_store_and_retrieve_secret() {
    let manager = SecretManager::new(SecretPolicyConfig::default());

    let result = manager
        .store_secret(
            "api-key".to_string(),
            "supersecretkey123!".to_string(),
            None,
        )
        .await;
    assert!(result.is_ok());

    let value = manager.get_secret("api-key").await.unwrap();
    assert_eq!(value, Some("supersecretkey123!".to_string()));
}

#[tokio::test]
async fn test_secret_not_found() {
    let manager = SecretManager::new(SecretPolicyConfig::default());

    let value = manager.get_secret("nonexistent").await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_delete_secret() {
    let manager = SecretManager::new(SecretPolicyConfig::default());

    manager
        .store_secret("temp-key".to_string(), "temp_value_1234!".to_string(), None)
        .await
        .unwrap();

    let result = manager.delete_secret("temp-key").await.unwrap();
    assert!(result);

    let value = manager.get_secret("temp-key").await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_validate_secret_length() {
    let policy = SecretPolicyConfig {
        min_length: 16,
        ..Default::default()
    };
    let manager = SecretManager::new(policy);

    let result = manager
        .store_secret("short-key".to_string(), "tooshort".to_string(), None)
        .await;

    assert!(result.is_err());
}

// =============================================================================
// AuditLogger Tests
// =============================================================================

#[tokio::test]
async fn test_log_event() {
    let logger = AuditLogger::default();

    let event = SecurityEvent {
        id: "event-1".to_string(),
        timestamp: std::time::SystemTime::now(),
        event_type: SecurityEventType::AccessRequest,
        principal: Principal::User("test-user".to_string()),
        resource: Some("resource:1".to_string()),
        action: Some("read".to_string()),
        result: EventResult::Allowed,
        reason: None,
        details: HashMap::new(),
    };

    let event_id = logger.log_event(event.clone()).await;
    assert_eq!(event_id, "event-1");

    let query = LogQuery::default();
    let events = logger.query(query).await;
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_query_by_principal() {
    let logger = AuditLogger::default();

    logger
        .log_event(SecurityEvent {
            id: "1".to_string(),
            timestamp: std::time::SystemTime::now(),
            event_type: SecurityEventType::AccessRequest,
            principal: Principal::User("user1".to_string()),
            resource: None,
            action: None,
            result: EventResult::Allowed,
            reason: None,
            details: HashMap::new(),
        })
        .await;

    logger
        .log_event(SecurityEvent {
            id: "2".to_string(),
            timestamp: std::time::SystemTime::now(),
            event_type: SecurityEventType::AccessRequest,
            principal: Principal::User("user2".to_string()),
            resource: None,
            action: None,
            result: EventResult::Allowed,
            reason: None,
            details: HashMap::new(),
        })
        .await;

    let query = LogQuery {
        principal: Some(Principal::User("user1".to_string())),
        ..Default::default()
    };

    let events = logger.query(query).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "1");
}
