//! Agent Variants
//!
//! Design Reference: docs/03-module-design/agent/agent-variants.md
//!
//! This module provides agent variant management, including:
//! - Loading agent definitions with variant support
//! - Variant inheritance and override mechanisms
//! - Agent reference resolution (e.g., "code-reviewer:quick")

pub mod types;
pub mod registry;

pub use types::AgentVariantError;
pub use registry::AgentVariantRegistryImpl;

// Re-export types for convenience
pub use types::{
    ModelConfig, PermissionConfig, AgentDefinition, AgentVariant, VariantOverrides,
    VariantInfo, AgentVariantInfo, ValidationResult, ResolvedAgentRef,
};
