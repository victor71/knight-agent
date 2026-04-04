//! Security Manager
//!
//! Design Reference: docs/03-module-design/security/security-manager.md
//!
//! A comprehensive security management system for Knight-Agent.
//!
//! # Features
//!
//! - Permission management (grant, revoke, check)
//! - Security policy engine (RBAC, ABAC)
//! - Audit logging and event tracking
//! - Secret/key management
//! - Threat detection
//!
//! # Example
//!
//! ```rust,no_run
//! use security_manager::{SecurityManagerImpl, SecurityManager};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let security = SecurityManagerImpl::new()?;
//!     security.init().await?;
//!
//!     let (allowed, reason) = security
//!         .check_permission("user:admin", "file:/project/**", "read", None)
//!         .await?;
//!
//!     println!("Permission allowed: {}, reason: {:?}", allowed, reason);
//!     Ok(())
//! }
//! ```

pub mod audit;
pub mod policy;
pub mod secrets;
pub mod system;
pub mod types;

pub use system::{SecurityManager, SecurityManagerImpl};

#[derive(thiserror::Error, Debug)]
pub enum SecurityError {
    #[error("Security manager not initialized")]
    NotInitialized,

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Authorization failed: {0}")]
    AuthzFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Secret error: {0}")]
    SecretError(String),

    #[error("Threat detected: {0}")]
    ThreatDetected(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;

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
}
