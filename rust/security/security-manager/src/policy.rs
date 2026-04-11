//! Policy Engine
//!
//! Handles security policy evaluation and management.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::types::*;

/// Policy engine for evaluating access requests
pub struct PolicyEngine {
    policies: Arc<RwLock<HashMap<String, SecurityPolicy>>>,
    default_policy: DefaultPolicy,
}

impl PolicyEngine {
    pub fn new(default_policy: DefaultPolicy) -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            default_policy,
        }
    }

    /// Create a new policy engine with default configuration
    pub fn with_default() -> Self {
        Self::new(DefaultPolicy::Deny)
    }

    /// Add a policy
    pub async fn add_policy(&self, policy: SecurityPolicy) {
        let mut policies = self.policies.write().await;
        policies.insert(policy.id.clone(), policy);
    }

    /// Remove a policy
    pub async fn remove_policy(&self, policy_id: &str) -> Option<SecurityPolicy> {
        let mut policies = self.policies.write().await;
        policies.remove(policy_id)
    }

    /// Get a policy by ID
    pub async fn get_policy(&self, policy_id: &str) -> Option<SecurityPolicy> {
        let policies = self.policies.read().await;
        policies.get(policy_id).cloned()
    }

    /// List all policies, optionally filtered by type
    pub async fn list_policies(&self, policy_type: Option<PolicyType>) -> Vec<SecurityPolicy> {
        let policies = self.policies.read().await;
        let mut result: Vec<_> = policies.values().cloned().collect();

        if let Some(pt) = policy_type {
            result.retain(|p| p.policy_type == pt);
        }

        // Sort by priority (higher first)
        result.sort_by(|a, b| b.priority.cmp(&a.priority));
        result
    }

    /// Evaluate a policy for the given context
    pub async fn evaluate(
        &self,
        policy_id: &str,
        context: &SecurityContext,
    ) -> Option<PolicyEvaluationResult> {
        let policies = self.policies.read().await;
        let policy = policies.get(policy_id)?;

        if !policy.enabled {
            return Some(PolicyEvaluationResult {
                allowed: false,
                matched_policy: Some(policy_id.to_string()),
                matched_rule: None,
                reason: "policy_disabled".to_string(),
            });
        }

        // Evaluate rules in priority order
        for rule in &policy.rules {
            if let Some(result) = self.evaluate_rule(rule, context) {
                return Some(result);
            }
        }

        // No rule matched - use default
        Some(PolicyEvaluationResult {
            allowed: matches!(self.default_policy, DefaultPolicy::Allow),
            matched_policy: Some(policy_id.to_string()),
            matched_rule: None,
            reason: "default_policy".to_string(),
        })
    }

    /// Evaluate a single rule against the context
    fn evaluate_rule(
        &self,
        rule: &PolicyRule,
        context: &SecurityContext,
    ) -> Option<PolicyEvaluationResult> {
        // Check principal match
        if let Some(ref pattern) = rule.principal {
            if !self.match_principal(pattern, context) {
                return None;
            }
        }

        // Check resource match
        if let Some(ref pattern) = rule.resource {
            if !self.match_pattern(
                pattern,
                context
                    .metadata
                    .get("resource")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            ) {
                return None;
            }
        }

        // Check action match
        if let Some(ref pattern) = rule.action {
            let action = context
                .metadata
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !self.match_pattern(pattern, action) {
                return None;
            }
        }

        // Check conditions
        for condition in &rule.conditions {
            if !self.evaluate_condition(condition, context) {
                return None;
            }
        }

        // Rule matched
        Some(PolicyEvaluationResult {
            allowed: matches!(rule.effect, PolicyEffect::Allow),
            matched_policy: None,
            matched_rule: Some(rule.name.clone()),
            reason: format!("rule_matched:{}", rule.name),
        })
    }

    /// Match principal against a pattern
    fn match_principal(&self, pattern: &str, context: &SecurityContext) -> bool {
        match context.principal {
            Principal::User(ref id) => {
                pattern == "*" || pattern == "user:*" || pattern == format!("user:{}", id).as_str()
            }
            Principal::Agent(ref id) => {
                pattern == "*"
                    || pattern == "agent:*"
                    || pattern == format!("agent:{}", id).as_str()
            }
            Principal::Session(ref id) => {
                pattern == "*"
                    || pattern == "session:*"
                    || pattern == format!("session:{}", id).as_str()
            }
        }
    }

    /// Match a value against a pattern (supports * wildcards)
    fn match_pattern(&self, pattern: &str, value: &str) -> bool {
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

    /// Evaluate a condition against the context
    fn evaluate_condition(&self, condition: &Condition, context: &SecurityContext) -> bool {
        match condition.condition_type {
            ConditionType::Workspace => {
                if let Some(workspace) = &context.workspace {
                    self.evaluate_operator(&condition.operator, &condition.value, workspace)
                } else {
                    false
                }
            }
            ConditionType::Time => {
                // Time conditions would be evaluated against current time
                // For now, just return true
                true
            }
            ConditionType::Ip => {
                if let Some(ip) = &context.ip_address {
                    self.evaluate_operator(&condition.operator, &condition.value, ip)
                } else {
                    false
                }
            }
            ConditionType::Custom => {
                // Custom conditions are evaluated by checking metadata
                let key = condition
                    .value
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let expected = condition
                    .value
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if let Some(actual) = context.metadata.get(key).and_then(|v| v.as_str()) {
                    self.evaluate_operator(
                        &condition.operator,
                        &serde_json::json!(expected),
                        actual,
                    )
                } else {
                    false
                }
            }
        }
    }

    /// Evaluate a condition operator
    fn evaluate_operator(
        &self,
        op: &ConditionOperator,
        expected: &serde_json::Value,
        actual: &str,
    ) -> bool {
        match op {
            ConditionOperator::Equals => {
                if let Some(exp_str) = expected.as_str() {
                    actual == exp_str
                } else {
                    false
                }
            }
            ConditionOperator::Contains => {
                if let Some(exp_str) = expected.as_str() {
                    actual.contains(exp_str)
                } else {
                    false
                }
            }
            ConditionOperator::Matches => {
                if let Some(pattern) = expected.as_str() {
                    // Simple regex match - in production, use regex crate
                    self.match_pattern(pattern, actual)
                } else {
                    false
                }
            }
            ConditionOperator::InRange => {
                // Check if actual is in a list of values
                if let Some(arr) = expected.as_array() {
                    arr.iter().any(|v| v.as_str() == Some(actual))
                } else {
                    false
                }
            }
        }
    }
}
