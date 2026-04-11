//! Agent Variant Registry
//!
//! Implementation of the agent variant registry.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info};

use crate::types::*;

/// Agent variant registry implementation
pub struct AgentVariantRegistryImpl {
    /// Registry name
    #[allow(dead_code)]
    name: String,
    /// Whether the registry is initialized
    initialized: bool,
    /// Stored agent definitions (agent_id -> definition)
    agents: Arc<AsyncRwLock<HashMap<String, AgentDefinition>>>,
    /// Variant mappings (agent_id -> variant_name -> variant)
    variants: Arc<AsyncRwLock<HashMap<String, HashMap<String, AgentVariant>>>>,
}

impl AgentVariantRegistryImpl {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            name: "agent-variants".to_string(),
            initialized: false,
            agents: Arc::new(AsyncRwLock::new(HashMap::new())),
            variants: Arc::new(AsyncRwLock::new(HashMap::new())),
        }
    }

    /// Initialize the registry
    pub async fn initialize(&mut self) -> VariantResult<()> {
        self.initialized = true;
        info!("AgentVariantRegistry initialized");
        Ok(())
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Register an agent definition
    pub async fn register_agent(&self, definition: AgentDefinition) -> VariantResult<()> {
        let agent_id = definition.id.clone();
        let mut agents = self.agents.write().await;
        agents.insert(agent_id.clone(), definition.clone());

        // Also register variants if present
        if !definition.variants.is_empty() {
            let mut variants = self.variants.write().await;
            let variant_map = variants
                .entry(agent_id.clone())
                .or_insert_with(HashMap::new);
            for variant in definition.variants {
                variant_map.insert(variant.name.clone(), variant);
            }
        }

        debug!("Registered agent: {}", agent_id);
        Ok(())
    }

    /// Get an agent definition (without variant resolution)
    pub async fn get_agent(&self, agent_id: &str) -> VariantResult<AgentDefinition> {
        let agents = self.agents.read().await;
        agents
            .get(agent_id)
            .cloned()
            .ok_or_else(|| AgentVariantError::AgentNotFound(agent_id.to_string()))
    }

    /// Get a specific variant of an agent
    pub async fn get_variant(
        &self,
        agent_id: &str,
        variant_name: &str,
    ) -> VariantResult<AgentVariant> {
        let variants = self.variants.read().await;
        variants
            .get(agent_id)
            .and_then(|v| v.get(variant_name))
            .cloned()
            .ok_or_else(|| AgentVariantError::NotFound(format!("{}:{}", agent_id, variant_name)))
    }

    /// List all variants for an agent
    pub async fn list_variants(&self, agent_id: &str) -> VariantResult<Vec<VariantInfo>> {
        let variants = self.variants.read().await;
        let variant_map = variants
            .get(agent_id)
            .ok_or_else(|| AgentVariantError::AgentNotFound(agent_id.to_string()))?;

        let infos: Vec<VariantInfo> = variant_map
            .values()
            .map(|v| VariantInfo {
                name: v.name.clone(),
                description: v.description.clone(),
                extends: v.extends.clone(),
            })
            .collect();

        Ok(infos)
    }

    /// List all registered agents with their variants
    pub async fn list_all_agents(&self) -> VariantResult<Vec<AgentVariantInfo>> {
        let agents = self.agents.read().await;
        let variants = self.variants.read().await;

        let mut result = Vec::new();
        for (agent_id, agent_def) in agents.iter() {
            let variant_map = variants.get(agent_id);
            let variant_infos: Vec<VariantInfo> = variant_map
                .map(|v| {
                    v.values()
                        .map(|var| VariantInfo {
                            name: var.name.clone(),
                            description: var.description.clone(),
                            extends: var.extends.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            result.push(AgentVariantInfo {
                agent_id: agent_id.clone(),
                name: agent_def.name.clone(),
                default_variant: agent_def.variant.clone(),
                variants: variant_infos,
            });
        }

        Ok(result)
    }

    /// Load and resolve an agent with optional variant
    pub async fn load_agent_definition(
        &self,
        agent_id: &str,
        variant_name: Option<&str>,
    ) -> VariantResult<AgentDefinition> {
        // First get the base agent
        let base_agent = self.get_agent(agent_id).await?;

        // If no variant specified, return base
        let variant_name = match variant_name {
            Some(v) => v,
            None => return Ok(base_agent),
        };

        // Get the variant
        let variant = self.get_variant(agent_id, variant_name).await?;

        // Resolve the variant (apply overrides)
        self.resolve_variant(base_agent, &variant).await
    }

    /// Resolve a variant by applying overrides to base definition
    async fn resolve_variant(
        &self,
        base: AgentDefinition,
        variant: &AgentVariant,
    ) -> VariantResult<AgentDefinition> {
        let mut resolved = base;
        resolved.variant = Some(variant.name.clone());

        // Apply overrides
        if !variant.overrides.is_default() {
            let overrides = &variant.overrides;

            if let Some(model) = &overrides.model {
                resolved.model = model.clone();
            }
            if let Some(instructions) = &overrides.instructions {
                resolved.instructions = instructions.clone();
            }
            if let Some(tools) = &overrides.tools {
                resolved.tools = tools.clone();
            }
            if let Some(skills) = &overrides.skills {
                resolved.skills = skills.clone();
            }
            if let Some(capabilities) = &overrides.capabilities {
                resolved.capabilities = capabilities.clone();
            }
            if let Some(permissions) = &overrides.permissions {
                resolved.permissions = permissions.clone();
            }
        }

        Ok(resolved)
    }

    /// Validate an agent and its variants
    pub async fn validate_agent(&self, agent_id: &str) -> VariantResult<ValidationResult> {
        let agent = self.get_agent(agent_id).await?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate base agent
        if agent.id.is_empty() {
            errors.push("Agent ID cannot be empty".to_string());
        }
        if agent.name.is_empty() {
            errors.push("Agent name cannot be empty".to_string());
        }
        if agent.role.is_empty() {
            warnings.push("Agent role is empty".to_string());
        }

        // Validate model config
        if agent.model.model.is_empty() {
            warnings.push("Model name is empty".to_string());
        }

        // Validate variants
        let variants = self.variants.read().await;
        if let Some(variant_map) = variants.get(agent_id) {
            for (name, variant) in variant_map {
                if name.is_empty() {
                    errors.push("Variant name cannot be empty".to_string());
                }
                // Check extends exists if specified
                if let Some(extends) = &variant.extends {
                    if extends.is_empty() {
                        errors.push(format!("Variant {} has empty extends", name));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(ValidationResult::valid().with_warnings(warnings))
        } else {
            Ok(ValidationResult::invalid(errors).with_warnings(warnings))
        }
    }

    /// Validate a specific variant
    pub async fn validate_variant(
        &self,
        agent_id: &str,
        variant_name: &str,
    ) -> VariantResult<ValidationResult> {
        // First check the variant exists
        let _variant = self.get_variant(agent_id, variant_name).await?;

        // Basic validation
        let mut errors = Vec::new();
        let warnings = Vec::new();

        if variant_name.is_empty() {
            errors.push("Variant name cannot be empty".to_string());
        }

        // Check for circular inheritance would go here
        // For now, just do basic checks

        if errors.is_empty() {
            Ok(ValidationResult::valid().with_warnings(warnings))
        } else {
            Ok(ValidationResult::invalid(errors).with_warnings(warnings))
        }
    }

    /// Create a new variant for an agent
    pub async fn create_variant(&self, agent_id: &str, variant: AgentVariant) -> VariantResult<()> {
        // Verify agent exists
        let _ = self.get_agent(agent_id).await?;

        let mut variants = self.variants.write().await;
        let variant_map = variants
            .entry(agent_id.to_string())
            .or_insert_with(HashMap::new);
        variant_map.insert(variant.name.clone(), variant);

        info!("Created variant for agent: {}", agent_id);
        Ok(())
    }

    /// Delete a variant
    pub async fn delete_variant(&self, agent_id: &str, variant_name: &str) -> VariantResult<()> {
        let mut variants = self.variants.write().await;
        let variant_map = variants
            .get_mut(agent_id)
            .ok_or_else(|| AgentVariantError::AgentNotFound(agent_id.to_string()))?;

        variant_map
            .remove(variant_name)
            .ok_or_else(|| AgentVariantError::NotFound(format!("{}:{}", agent_id, variant_name)))?;

        info!("Deleted variant {} for agent: {}", variant_name, agent_id);
        Ok(())
    }

    /// Resolve an agent reference string (e.g., "code-reviewer" or "code-reviewer:quick")
    #[allow(dead_code)]
    pub fn resolve_agent_ref(agent_ref: &str) -> VariantResult<ResolvedAgentRef> {
        ResolvedAgentRef::parse(agent_ref)
    }

    /// Check if an agent exists
    pub async fn has_agent(&self, agent_id: &str) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(agent_id)
    }

    /// Check if a variant exists
    pub async fn has_variant(&self, agent_id: &str, variant_name: &str) -> bool {
        let variants = self.variants.read().await;
        variants
            .get(agent_id)
            .map(|v| v.contains_key(variant_name))
            .unwrap_or(false)
    }
}

impl Default for AgentVariantRegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}
