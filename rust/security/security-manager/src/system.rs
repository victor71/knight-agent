//! Security Manager System
//!
//! Main implementation of the security manager.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::RwLock;

use crate::audit::AuditLogger;
use crate::policy::PolicyEngine;
use crate::secrets::SecretManager;
use crate::types::*;
use crate::SecurityError;

/// Security manager trait
#[allow(async_fn_in_trait)]
pub trait SecurityManager: Send + Sync {
    fn new() -> Result<Self, SecurityError>
    where
        Self: Sized;

    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;

    // Permission management
    async fn grant_permission(&self, grant: PermissionGrant) -> Result<bool, SecurityError>;
    async fn revoke_permission(&self, principal: &str, resource: &str, action: &str) -> Result<bool, SecurityError>;
    async fn check_permission(&self, principal: &str, resource: &str, action: &str, context: Option<SecurityContext>) -> Result<(bool, Option<String>), SecurityError>;
    async fn list_permissions(&self, principal: &str) -> Result<Vec<Permission>, SecurityError>;

    // Policy management
    async fn create_policy(&self, policy: SecurityPolicy) -> Result<String, SecurityError>;
    async fn update_policy(&self, policy_id: &str, policy: SecurityPolicy) -> Result<bool, SecurityError>;
    async fn delete_policy(&self, policy_id: &str) -> Result<bool, SecurityError>;
    async fn get_policy(&self, policy_id: &str) -> Result<Option<SecurityPolicy>, SecurityError>;
    async fn list_policies(&self, policy_type: Option<PolicyType>) -> Result<Vec<SecurityPolicy>, SecurityError>;
    async fn evaluate_policy(&self, policy_id: &str, context: SecurityContext) -> Result<PolicyEvaluationResult, SecurityError>;

    // Audit logging
    async fn log_event(&self, event: SecurityEvent) -> Result<String, SecurityError>;
    async fn query_logs(&self, query: LogQuery) -> Result<Vec<SecurityEvent>, SecurityError>;
    async fn get_log_summary(&self, time_range: Option<TimeRange>) -> Result<LogSummary, SecurityError>;

    // Secret management
    async fn store_secret(&self, key: &str, value: &str, metadata: Option<HashMap<String, serde_json::Value>>) -> Result<bool, SecurityError>;
    async fn get_secret(&self, key: &str) -> Result<Option<String>, SecurityError>;
    async fn delete_secret(&self, key: &str) -> Result<bool, SecurityError>;
    async fn list_secrets(&self) -> Result<Vec<SecretInfo>, SecurityError>;
    async fn rotate_secret(&self, key: &str, new_value: &str) -> Result<bool, SecurityError>;

    // Threat detection
    async fn analyze_threats(&self, time_range: Option<TimeRange>) -> Result<Vec<ThreatInfo>, SecurityError>;
    async fn is_suspicious(&self, activity: Activity) -> Result<(bool, f32, Vec<String>), SecurityError>;

    // Configuration
    async fn get_security_config(&self) -> Result<SecurityConfig, SecurityError>;
    async fn update_security_config(&self, config: SecurityConfig) -> Result<bool, SecurityError>;
}

/// Main security manager implementation
pub struct SecurityManagerImpl {
    name: String,
    initialized: AtomicBool,
    config: RwLock<SecurityConfig>,

    // Core components
    policy_engine: PolicyEngine,
    audit_logger: AuditLogger,
    secret_manager: SecretManager,

    // Permission storage
    permissions: RwLock<HashMap<String, Permission>>,

    // Threat detection state
    threat_history: RwLock<Vec<ThreatInfo>>,
}

impl SecurityManagerImpl {
    /// Create a new security manager
    pub fn new() -> Result<Self, SecurityError> {
        Ok(Self {
            name: "security-manager".to_string(),
            initialized: AtomicBool::new(false),
            config: RwLock::new(SecurityConfig::default()),
            policy_engine: PolicyEngine::with_default(),
            audit_logger: AuditLogger::default(),
            secret_manager: SecretManager::new(SecretPolicyConfig::default()),
            permissions: RwLock::new(HashMap::new()),
            threat_history: RwLock::new(Vec::new()),
        })
    }

    /// Initialize the security manager
    pub async fn init(&self) -> Result<(), SecurityError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Log a security event
    pub async fn log_security_event(&self, event_type: SecurityEventType, principal: Principal, resource: Option<String>, action: Option<String>, result: EventResult, reason: Option<String>) {
        let event = SecurityEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            event_type,
            principal,
            resource,
            action,
            result,
            reason,
            details: HashMap::new(),
        };

        let _ = self.log_event(event).await;
    }
}

impl SecurityManager for SecurityManagerImpl {
    fn new() -> Result<Self, SecurityError> {
        Self::new()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    async fn grant_permission(&self, grant: PermissionGrant) -> Result<bool, SecurityError> {
        let permission = Permission {
            id: uuid::Uuid::new_v4().to_string(),
            principal: grant.principal.clone(),
            resource: grant.resource.clone(),
            actions: grant.actions.clone(),
            conditions: grant.conditions.clone(),
            granted_at: std::time::SystemTime::now(),
            expires_at: grant.expires_at,
        };

        let key = format!("{}:{}", permission.principal, permission.resource);
        let mut permissions = self.permissions.write().await;
        permissions.insert(key, permission);

        // Log the event
        self.log_security_event(
            SecurityEventType::PermissionGranted,
            grant.principal,
            Some(grant.resource),
            None,
            EventResult::Allowed,
            None,
        ).await;

        Ok(true)
    }

    async fn revoke_permission(&self, principal: &str, resource: &str, _action: &str) -> Result<bool, SecurityError> {
        let key = format!("{}:{}", principal, resource);
        let mut permissions = self.permissions.write().await;
        let removed = permissions.remove(&key).is_some();

        if removed {
            let principal = if principal.starts_with("user:") {
                Principal::User(principal.trim_start_matches("user:").to_string())
            } else if principal.starts_with("agent:") {
                Principal::Agent(principal.trim_start_matches("agent:").to_string())
            } else {
                Principal::User(principal.to_string())
            };

            self.log_security_event(
                SecurityEventType::PermissionRevoked,
                principal,
                Some(resource.to_string()),
                None,
                EventResult::Allowed,
                None,
            ).await;
        }

        Ok(removed)
    }

    async fn check_permission(&self, principal: &str, resource: &str, action: &str, context: Option<SecurityContext>) -> Result<(bool, Option<String>), SecurityError> {
        // Build principal enum
        let principal = if principal.starts_with("user:") {
            Principal::User(principal.trim_start_matches("user:").to_string())
        } else if principal.starts_with("agent:") {
            Principal::Agent(principal.trim_start_matches("agent:").to_string())
        } else {
            Principal::User(principal.to_string())
        };

        // Create security context
        let ctx = context.unwrap_or_else(|| SecurityContext {
            principal: principal.clone(),
            session_id: None,
            agent_id: None,
            workspace: None,
            ip_address: None,
            timestamp: std::time::SystemTime::now(),
            metadata: HashMap::new(),
        });

        // Add resource and action to metadata
        let mut metadata = ctx.metadata.clone();
        metadata.insert("resource".to_string(), serde_json::json!(resource));
        metadata.insert("action".to_string(), serde_json::json!(action));
        let ctx = SecurityContext { metadata, ..ctx };

        // Check direct permissions by iterating (for wildcard matching)
        let permissions = self.permissions.read().await;
        let principal_str = principal.to_string();

        for permission in permissions.values() {
            let perm_principal_str = permission.principal.to_string();

            // Check if principal matches
            if perm_principal_str != "*" && perm_principal_str != principal_str {
                continue;
            }

            // Check if resource matches (supports wildcards)
            if permission.resource != "*" && !Self::match_pattern(&permission.resource, resource) {
                continue;
            }

            // Check if action is allowed
            if permission.actions.contains(&action.to_string()) || permission.actions.contains(&"*".to_string()) {
                self.log_security_event(
                    SecurityEventType::AccessRequest,
                    principal.clone(),
                    Some(resource.to_string()),
                    Some(action.to_string()),
                    EventResult::Allowed,
                    Some("permission_granted".to_string()),
                ).await;

                return Ok((true, Some("permission_granted".to_string())));
            }
        }

        // Check policies
        let policies = self.policy_engine.list_policies(None).await;
        for policy in policies {
            if let Some(result) = self.policy_engine.evaluate(&policy.id, &ctx).await {
                self.log_security_event(
                    if result.allowed {
                        SecurityEventType::AccessRequest
                    } else {
                        SecurityEventType::AccessDenied
                    },
                    principal.clone(),
                    Some(resource.to_string()),
                    Some(action.to_string()),
                    if result.allowed { EventResult::Allowed } else { EventResult::Denied },
                    Some(result.reason.clone()),
                ).await;

                return Ok((result.allowed, Some(result.reason)));
            }
        }

        // Default deny
        self.log_security_event(
            SecurityEventType::AccessDenied,
            principal,
            Some(resource.to_string()),
            Some(action.to_string()),
            EventResult::Denied,
            Some("default_deny".to_string()),
        ).await;

        Ok((false, Some("default_deny".to_string())))
    }

    async fn list_permissions(&self, principal: &str) -> Result<Vec<Permission>, SecurityError> {
        let permissions = self.permissions.read().await;
        let result: Vec<_> = permissions
            .values()
            .filter(|p| p.principal.id() == principal)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn create_policy(&self, policy: SecurityPolicy) -> Result<String, SecurityError> {
        let policy_id = policy.id.clone();
        self.policy_engine.add_policy(policy).await;
        Ok(policy_id)
    }

    async fn update_policy(&self, policy_id: &str, policy: SecurityPolicy) -> Result<bool, SecurityError> {
        let existing = self.policy_engine.get_policy(policy_id).await;
        if existing.is_none() {
            return Ok(false);
        }

        self.policy_engine.add_policy(policy).await;
        Ok(true)
    }

    async fn delete_policy(&self, policy_id: &str) -> Result<bool, SecurityError> {
        Ok(self.policy_engine.remove_policy(policy_id).await.is_some())
    }

    async fn get_policy(&self, policy_id: &str) -> Result<Option<SecurityPolicy>, SecurityError> {
        Ok(self.policy_engine.get_policy(policy_id).await)
    }

    async fn list_policies(&self, policy_type: Option<PolicyType>) -> Result<Vec<SecurityPolicy>, SecurityError> {
        Ok(self.policy_engine.list_policies(policy_type).await)
    }

    async fn evaluate_policy(&self, policy_id: &str, context: SecurityContext) -> Result<PolicyEvaluationResult, SecurityError> {
        match self.policy_engine.evaluate(policy_id, &context).await {
            Some(result) => Ok(result),
            None => Err(SecurityError::PolicyNotFound(policy_id.to_string())),
        }
    }

    async fn log_event(&self, event: SecurityEvent) -> Result<String, SecurityError> {
        Ok(self.audit_logger.log_event(event).await)
    }

    async fn query_logs(&self, query: LogQuery) -> Result<Vec<SecurityEvent>, SecurityError> {
        Ok(self.audit_logger.query(query).await)
    }

    async fn get_log_summary(&self, time_range: Option<TimeRange>) -> Result<LogSummary, SecurityError> {
        Ok(self.audit_logger.get_summary(time_range).await)
    }

    async fn store_secret(&self, key: &str, value: &str, metadata: Option<HashMap<String, serde_json::Value>>) -> Result<bool, SecurityError> {
        self.secret_manager
            .store_secret(key.to_string(), value.to_string(), metadata)
            .await
            .map_err(|e| SecurityError::SecretError(e.to_string()))
    }

    async fn get_secret(&self, key: &str) -> Result<Option<String>, SecurityError> {
        self.secret_manager
            .get_secret(key)
            .await
            .map_err(|e| SecurityError::SecretError(e.to_string()))
    }

    async fn delete_secret(&self, key: &str) -> Result<bool, SecurityError> {
        self.secret_manager
            .delete_secret(key)
            .await
            .map_err(|e| SecurityError::SecretError(e.to_string()))
    }

    async fn list_secrets(&self) -> Result<Vec<SecretInfo>, SecurityError> {
        Ok(self.secret_manager.list_secrets().await)
    }

    async fn rotate_secret(&self, key: &str, new_value: &str) -> Result<bool, SecurityError> {
        self.secret_manager
            .rotate_secret(key, new_value.to_string())
            .await
            .map_err(|e| SecurityError::SecretError(e.to_string()))
    }

    async fn analyze_threats(&self, _time_range: Option<TimeRange>) -> Result<Vec<ThreatInfo>, SecurityError> {
        let history = self.threat_history.read().await;
        Ok(history.clone())
    }

    async fn is_suspicious(&self, activity: Activity) -> Result<(bool, f32, Vec<String>), SecurityError> {
        let mut reasons = Vec::new();
        let mut confidence = 0.0;

        // Simple threat detection heuristics
        let history = self.threat_history.read().await;

        // Check for repeated denied accesses
        let denied_count = history
            .iter()
            .filter(|t| t.affected_principals.contains(&activity.principal.to_string()))
            .count();

        if denied_count > 5 {
            confidence += 0.3;
            reasons.push("repeated_denied_accesses".to_string());
        }

        // Check for rapid activity
        if history.len() > 100 {
            confidence += 0.2;
            reasons.push("high_activity_volume".to_string());
        }

        let is_suspicious = confidence > 0.5;
        Ok((is_suspicious, confidence, reasons))
    }

    async fn get_security_config(&self) -> Result<SecurityConfig, SecurityError> {
        let config = self.config.read().await;
        Ok(config.clone())
    }

    async fn update_security_config(&self, config: SecurityConfig) -> Result<bool, SecurityError> {
        let mut current = self.config.write().await;
        *current = config;
        Ok(true)
    }
}

impl SecurityManagerImpl {
    /// Match a value against a pattern (supports * wildcards)
    fn match_pattern(pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Simple wildcard matching
        let pattern_parts: Vec<&str> = pattern.split('*').collect();
        if pattern_parts.len() == 1 {
            return pattern == value;
        }

        let value_lower = value.to_lowercase();
        let pattern_lower = pattern.to_lowercase();
        let mut pos = 0;

        for part in &pattern_parts {
            if part.is_empty() {
                continue;
            }
            if let Some(idx) = value_lower[pos..].find(&part.to_lowercase()) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }

        // If pattern ends with *, we're good
        // If pattern starts with *, we already matched
        // If pattern has * in middle, we need to ensure we reached the end
        if !pattern_lower.ends_with('*') && pos != value_lower.len() {
            return false;
        }

        true
    }
}

impl Default for SecurityManagerImpl {
    fn default() -> Self {
        Self::new().expect("failed to create security manager")
    }
}
