//! Agent Variants Types
//!
//! Core data types for agent variant management.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Agent variant errors
#[derive(Error, Debug)]
pub enum AgentVariantError {
    #[error("Variant not found: {0}")]
    NotFound(String),
    #[error("Variant registration failed: {0}")]
    RegistrationFailed(String),
    #[error("Variant validation failed: {0}")]
    ValidationFailed(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Invalid reference: {0}")]
    InvalidReference(String),
}

/// Result type for variant operations
pub type VariantResult<T> = Result<T, AgentVariantError>;

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_temperature() -> f64 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model: "claude".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
        }
    }
}

/// Permission configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionConfig {
    #[serde(default)]
    pub allow_read: bool,
    #[serde(default)]
    pub allow_write: bool,
    #[serde(default)]
    pub allow_execute: bool,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub denied_paths: Vec<String>,
}

/// Agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub role: String,
    pub model: ModelConfig,
    pub instructions: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub permissions: PermissionConfig,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub variants: Vec<AgentVariant>,
}

impl AgentDefinition {
    pub fn new(id: String, name: String, role: String) -> Self {
        Self {
            id,
            name,
            version: "1.0.0".to_string(),
            role,
            model: ModelConfig::default(),
            instructions: String::new(),
            tools: Vec::new(),
            skills: Vec::new(),
            capabilities: Vec::new(),
            permissions: PermissionConfig::default(),
            extends: None,
            variant: None,
            variants: Vec::new(),
        }
    }
}

/// Agent variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVariant {
    pub name: String,
    pub description: String,
    pub extends: Option<String>,
    #[serde(default)]
    pub overrides: VariantOverrides,
}

/// Variant overrides (partial definition to override parent)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VariantOverrides {
    #[serde(default)]
    pub model: Option<ModelConfig>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub permissions: Option<PermissionConfig>,
}

impl VariantOverrides {
    /// Check if all override fields are None (no overrides)
    pub fn is_default(&self) -> bool {
        self.model.is_none()
            && self.instructions.is_none()
            && self.tools.is_none()
            && self.skills.is_none()
            && self.capabilities.is_none()
            && self.permissions.is_none()
    }
}

/// Variant info for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantInfo {
    pub name: String,
    pub description: String,
    pub extends: Option<String>,
}

/// Agent variant info (for listing agents with variants)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVariantInfo {
    pub agent_id: String,
    pub name: String,
    pub default_variant: Option<String>,
    pub variants: Vec<VariantInfo>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }
}

/// Resolved agent reference
#[derive(Debug, Clone)]
pub struct ResolvedAgentRef {
    pub agent_id: String,
    pub variant: Option<String>,
}

impl ResolvedAgentRef {
    pub fn parse(agent_ref: &str) -> VariantResult<Self> {
        let parts: Vec<&str> = agent_ref.splitn(2, ':').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err(AgentVariantError::InvalidReference(
                "Empty agent reference".to_string(),
            ));
        }

        let agent_id = parts[0].to_string();
        let variant = if parts.len() > 1 && !parts[1].is_empty() {
            Some(parts[1].to_string())
        } else {
            None
        };

        Ok(Self { agent_id, variant })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_definition_new() {
        let def = AgentDefinition::new(
            "test".to_string(),
            "Test Agent".to_string(),
            "testing".to_string(),
        );
        assert_eq!(def.id, "test");
        assert_eq!(def.name, "Test Agent");
    }

    #[test]
    fn test_resolved_agent_ref_parse() {
        let ref1 = ResolvedAgentRef::parse("code-reviewer").unwrap();
        assert_eq!(ref1.agent_id, "code-reviewer");
        assert!(ref1.variant.is_none());

        let ref2 = ResolvedAgentRef::parse("code-reviewer:quick").unwrap();
        assert_eq!(ref2.agent_id, "code-reviewer");
        assert_eq!(ref2.variant, Some("quick".to_string()));
    }

    #[test]
    fn test_validation_result_valid() {
        let result = ValidationResult::valid();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_result_invalid() {
        let result = ValidationResult::invalid(vec!["Error 1".to_string()]);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }
}
