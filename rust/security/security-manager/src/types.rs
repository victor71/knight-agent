//! Security Manager Types
//!
//! Core data structures for the security system.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Permission principal type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum Principal {
    User(String),
    Agent(String),
    Session(String),
}

impl Principal {
    pub fn id(&self) -> &str {
        match self {
            Principal::User(id) => id,
            Principal::Agent(id) => id,
            Principal::Session(id) => id,
        }
    }
}

impl Default for Principal {
    fn default() -> Self {
        Principal::User(String::new())
    }
}

impl std::fmt::Display for Principal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Principal::User(id) => write!(f, "user:{}", id),
            Principal::Agent(id) => write!(f, "agent:{}", id),
            Principal::Session(id) => write!(f, "session:{}", id),
        }
    }
}

/// Permission action
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub principal: Principal,
    pub resource: String,
    pub actions: Vec<String>,
    pub conditions: Vec<Condition>,
    pub granted_at: std::time::SystemTime,
    pub expires_at: Option<std::time::SystemTime>,
}

/// Permission grant request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub principal: Principal,
    pub resource: String,
    pub actions: Vec<String>,
    pub conditions: Vec<Condition>,
    pub expires_at: Option<std::time::SystemTime>,
}

/// Condition for permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: ConditionType,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    Time,
    Ip,
    Workspace,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    Equals,
    Contains,
    Matches,
    InRange,
}

/// Security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub policy_type: PolicyType,
    pub enabled: bool,
    pub rules: Vec<PolicyRule>,
    pub priority: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyType {
    Rbac,
    Abac,
    Custom,
}

/// Policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub effect: PolicyEffect,
    pub principal: Option<String>,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Security context for authorization checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub principal: Principal,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub workspace: Option<String>,
    pub ip_address: Option<String>,
    pub timestamp: std::time::SystemTime,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            principal: Principal::default(),
            session_id: None,
            agent_id: None,
            workspace: None,
            ip_address: None,
            timestamp: std::time::SystemTime::UNIX_EPOCH,
            metadata: HashMap::new(),
        }
    }
}

/// Result of policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationResult {
    pub allowed: bool,
    pub matched_policy: Option<String>,
    pub matched_rule: Option<String>,
    pub reason: String,
}

/// Security event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    AccessRequest,
    AccessDenied,
    PermissionGranted,
    PermissionRevoked,
    PolicyViolation,
    ThreatDetected,
}

/// Security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: String,
    pub timestamp: std::time::SystemTime,
    pub event_type: SecurityEventType,
    pub principal: Principal,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub result: EventResult,
    pub reason: Option<String>,
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResult {
    Allowed,
    Denied,
}

/// Log query for audit logs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogQuery {
    pub time_range: Option<TimeRange>,
    pub event_types: Option<Vec<SecurityEventType>>,
    pub principal: Option<Principal>,
    pub resource: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Time range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: std::time::SystemTime,
    pub end: Option<std::time::SystemTime>,
}

/// Log summary statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogSummary {
    pub total_events: usize,
    pub by_event_type: HashMap<String, usize>,
    pub by_principal: HashMap<String, usize>,
    pub denied_count: usize,
    pub threat_count: usize,
}

/// Secret info (without value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    pub key: String,
    pub created_at: std::time::SystemTime,
    pub updated_at: std::time::SystemTime,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Activity for threat detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub principal: Principal,
    pub action: String,
    pub resource: String,
    pub context: SecurityContext,
}

/// Threat information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInfo {
    pub threat_type: String,
    pub confidence: f32,
    pub description: String,
    pub affected_principals: Vec<String>,
    pub detected_at: std::time::SystemTime,
    pub recommendations: Vec<String>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub default_policy: DefaultPolicy,
    pub audit: AuditConfig,
    pub threat_detection: ThreatDetectionConfig,
    pub secret_policy: SecretPolicyConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultPolicy {
    Allow,
    Deny,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_policy: DefaultPolicy::Deny,
            audit: AuditConfig::default(),
            threat_detection: ThreatDetectionConfig::default(),
            secret_policy: SecretPolicyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub log_all_events: bool,
    pub log_denied: bool,
    pub retention_days: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            log_all_events: true,
            log_denied: true,
            retention_days: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    pub enabled: bool,
    pub sensitivity: ThreatSensitivity,
    pub auto_block: bool,
}

impl Default for ThreatDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: ThreatSensitivity::Medium,
            auto_block: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatSensitivity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPolicyConfig {
    pub rotation_days: u32,
    pub min_length: usize,
    pub require_special_chars: bool,
}

impl Default for SecretPolicyConfig {
    fn default() -> Self {
        Self {
            rotation_days: 90,
            min_length: 16,
            require_special_chars: true,
        }
    }
}
