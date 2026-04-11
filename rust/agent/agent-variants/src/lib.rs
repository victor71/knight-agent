//! Agent Variants
//!
//! Design Reference: docs/03-module-design/agent/agent-variants.md
//!
//! This module provides agent variant management, including:
//! - Loading agent definitions with variant support
//! - Variant inheritance and override mechanisms
//! - Agent reference resolution (e.g., "code-reviewer:quick")

pub mod registry;
pub mod types;

pub use registry::AgentVariantRegistryImpl;
pub use types::AgentVariantError;

// Re-export types for convenience
pub use types::{
    AgentDefinition, AgentVariant, AgentVariantInfo, ModelConfig, PermissionConfig,
    ResolvedAgentRef, ValidationResult, VariantInfo, VariantOverrides,
};
