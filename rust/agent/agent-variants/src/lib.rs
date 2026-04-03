//! Agent Variants
//!
//! Design Reference: docs/03-module-design/agent/agent-variants.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentVariantError {
    #[error("Variant not found: {0}")]
    NotFound(String),
    #[error("Variant registration failed: {0}")]
    RegistrationFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVariant {
    pub name: String,
    pub variant_type: String,
    pub capabilities: Vec<String>,
}

pub trait AgentVariantRegistry: Send + Sync {
    fn new() -> Result<Self, AgentVariantError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn register_variant(&self, variant: AgentVariant) -> Result<(), AgentVariantError>;
    async fn get_variant(&self, name: &str) -> Result<AgentVariant, AgentVariantError>;
    async fn list_variants(&self) -> Result<Vec<AgentVariant>, AgentVariantError>;
}

pub struct AgentVariantRegistryImpl;

impl AgentVariantRegistry for AgentVariantRegistryImpl {
    fn new() -> Result<Self, AgentVariantError> {
        Ok(AgentVariantRegistryImpl)
    }

    fn name(&self) -> &str {
        "agent-variants"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn register_variant(&self, _variant: AgentVariant) -> Result<(), AgentVariantError> {
        Ok(())
    }

    async fn get_variant(&self, name: &str) -> Result<AgentVariant, AgentVariantError> {
        Err(AgentVariantError::NotFound(name.to_string()))
    }

    async fn list_variants(&self) -> Result<Vec<AgentVariant>, AgentVariantError> {
        Ok(vec![])
    }
}
