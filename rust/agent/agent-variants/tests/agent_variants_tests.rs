//! Agent Variants Tests
//!
//! Unit tests for the agent-variants module.

use agent_variants::{
    AgentDefinition, AgentVariant, AgentVariantError, AgentVariantRegistryImpl, ModelConfig,
    PermissionConfig, ResolvedAgentRef, ValidationResult, VariantOverrides,
};

#[tokio::test]
async fn test_registry_new() {
    let registry = AgentVariantRegistryImpl::new();
    assert!(!registry.is_initialized());
}

#[tokio::test]
async fn test_register_agent() {
    let registry = AgentVariantRegistryImpl::new();
    let def = AgentDefinition::new(
        "test-agent".to_string(),
        "Test Agent".to_string(),
        "testing".to_string(),
    );

    registry.register_agent(def.clone()).await.unwrap();
    assert!(registry.has_agent("test-agent").await);

    let loaded = registry.get_agent("test-agent").await.unwrap();
    assert_eq!(loaded.id, "test-agent");
    assert_eq!(loaded.name, "Test Agent");
}

#[tokio::test]
async fn test_get_agent_not_found() {
    let registry = AgentVariantRegistryImpl::new();
    let result = registry.get_agent("nonexistent").await;
    assert!(result.is_err());
    match result {
        Err(AgentVariantError::AgentNotFound(id)) => assert_eq!(id, "nonexistent"),
        _ => panic!("Expected AgentNotFound error"),
    }
}

#[tokio::test]
async fn test_register_and_get_variant() {
    let registry = AgentVariantRegistryImpl::new();

    // First register an agent
    let def = AgentDefinition::new(
        "code-reviewer".to_string(),
        "Code Reviewer".to_string(),
        "reviewing code".to_string(),
    );
    registry.register_agent(def).await.unwrap();

    // Create and register a variant
    let variant = AgentVariant {
        name: "quick".to_string(),
        description: "Quick code review".to_string(),
        extends: None,
        overrides: VariantOverrides::default(),
    };

    registry
        .create_variant("code-reviewer", variant)
        .await
        .unwrap();
    assert!(registry.has_variant("code-reviewer", "quick").await);

    let loaded_variant = registry
        .get_variant("code-reviewer", "quick")
        .await
        .unwrap();
    assert_eq!(loaded_variant.name, "quick");
}

#[tokio::test]
async fn test_list_variants() {
    let registry = AgentVariantRegistryImpl::new();

    // Register agent
    let def = AgentDefinition::new(
        "test-agent".to_string(),
        "Test Agent".to_string(),
        "testing".to_string(),
    );
    registry.register_agent(def).await.unwrap();

    // Register variants
    registry
        .create_variant(
            "test-agent",
            AgentVariant {
                name: "quick".to_string(),
                description: "Quick variant".to_string(),
                extends: None,
                overrides: VariantOverrides::default(),
            },
        )
        .await
        .unwrap();

    registry
        .create_variant(
            "test-agent",
            AgentVariant {
                name: "full".to_string(),
                description: "Full variant".to_string(),
                extends: None,
                overrides: VariantOverrides::default(),
            },
        )
        .await
        .unwrap();

    let variants = registry.list_variants("test-agent").await.unwrap();
    assert_eq!(variants.len(), 2);
    assert!(variants.iter().any(|v| v.name == "quick"));
    assert!(variants.iter().any(|v| v.name == "full"));
}

#[tokio::test]
async fn test_list_all_agents() {
    let registry = AgentVariantRegistryImpl::new();

    // Register multiple agents
    registry
        .register_agent(AgentDefinition::new(
            "agent1".to_string(),
            "Agent 1".to_string(),
            "role1".to_string(),
        ))
        .await
        .unwrap();

    registry
        .register_agent(AgentDefinition::new(
            "agent2".to_string(),
            "Agent 2".to_string(),
            "role2".to_string(),
        ))
        .await
        .unwrap();

    let agents = registry.list_all_agents().await.unwrap();
    assert_eq!(agents.len(), 2);
}

#[tokio::test]
async fn test_load_agent_definition_without_variant() {
    let registry = AgentVariantRegistryImpl::new();

    let def = AgentDefinition::new(
        "test-agent".to_string(),
        "Test Agent".to_string(),
        "testing".to_string(),
    );
    registry.register_agent(def.clone()).await.unwrap();

    let loaded = registry
        .load_agent_definition("test-agent", None)
        .await
        .unwrap();
    assert_eq!(loaded.id, "test-agent");
    assert!(loaded.variant.is_none());
}

#[tokio::test]
async fn test_load_agent_definition_with_variant() {
    let registry = AgentVariantRegistryImpl::new();

    // Register base agent
    let mut base_def = AgentDefinition::new(
        "code-reviewer".to_string(),
        "Code Reviewer".to_string(),
        "reviewing code".to_string(),
    );
    base_def.model = ModelConfig {
        provider: "anthropic".to_string(),
        model: "claude-sonnet".to_string(),
        temperature: 0.7,
        max_tokens: 4096,
    };
    base_def.instructions = "Review code for bugs".to_string();

    registry.register_agent(base_def).await.unwrap();

    // Register variant with overrides
    let variant = AgentVariant {
        name: "quick".to_string(),
        description: "Quick review".to_string(),
        extends: None,
        overrides: VariantOverrides {
            model: Some(ModelConfig {
                provider: "anthropic".to_string(),
                model: "claude-haiku".to_string(),
                temperature: 0.1,
                max_tokens: 2048,
            }),
            instructions: Some("Quick review only".to_string()),
            tools: Some(vec!["read".to_string()]),
            skills: None,
            capabilities: None,
            permissions: None,
        },
    };

    registry
        .create_variant("code-reviewer", variant)
        .await
        .unwrap();

    let loaded = registry
        .load_agent_definition("code-reviewer", Some("quick"))
        .await
        .unwrap();
    assert_eq!(loaded.variant, Some("quick".to_string()));
    assert_eq!(loaded.model.model, "claude-haiku");
    assert_eq!(loaded.instructions, "Quick review only");
    assert_eq!(loaded.tools, vec!["read".to_string()]);
}

#[tokio::test]
async fn test_validate_agent() {
    let registry = AgentVariantRegistryImpl::new();

    let def = AgentDefinition::new(
        "valid-agent".to_string(),
        "Valid Agent".to_string(),
        "testing".to_string(),
    );
    registry.register_agent(def).await.unwrap();

    let result = registry.validate_agent("valid-agent").await.unwrap();
    assert!(result.valid);
}

#[tokio::test]
async fn test_validate_agent_not_found() {
    let registry = AgentVariantRegistryImpl::new();
    let result = registry.validate_agent("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_variant() {
    let registry = AgentVariantRegistryImpl::new();

    // Register agent and variant
    registry
        .register_agent(AgentDefinition::new(
            "test-agent".to_string(),
            "Test Agent".to_string(),
            "testing".to_string(),
        ))
        .await
        .unwrap();

    registry
        .create_variant(
            "test-agent",
            AgentVariant {
                name: "to-delete".to_string(),
                description: "Will be deleted".to_string(),
                extends: None,
                overrides: VariantOverrides::default(),
            },
        )
        .await
        .unwrap();

    assert!(registry.has_variant("test-agent", "to-delete").await);

    registry
        .delete_variant("test-agent", "to-delete")
        .await
        .unwrap();
    assert!(!registry.has_variant("test-agent", "to-delete").await);
}

#[tokio::test]
async fn test_resolve_agent_ref() {
    let ref1 = ResolvedAgentRef::parse("code-reviewer").unwrap();
    assert_eq!(ref1.agent_id, "code-reviewer");
    assert!(ref1.variant.is_none());

    let ref2 = ResolvedAgentRef::parse("code-reviewer:quick").unwrap();
    assert_eq!(ref2.agent_id, "code-reviewer");
    assert_eq!(ref2.variant, Some("quick".to_string()));
}

#[tokio::test]
async fn test_resolve_agent_ref_invalid() {
    let result = ResolvedAgentRef::parse("");
    assert!(result.is_err());

    let result = ResolvedAgentRef::parse(":variant");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validation_result_valid() {
    let result = ValidationResult::valid();
    assert!(result.valid);
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_validation_result_invalid() {
    let result = ValidationResult::invalid(vec!["Error 1".to_string(), "Error 2".to_string()]);
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 2);
}

#[tokio::test]
async fn test_validation_result_with_warnings() {
    let result = ValidationResult::valid().with_warnings(vec!["Warning 1".to_string()]);
    assert!(result.valid);
    assert_eq!(result.warnings.len(), 1);
}

#[tokio::test]
async fn test_variant_overrides_is_default() {
    let empty = VariantOverrides::default();
    assert!(empty.is_default());

    let with_override = VariantOverrides {
        model: Some(ModelConfig::default()),
        ..Default::default()
    };
    assert!(!with_override.is_default());
}

#[tokio::test]
async fn test_model_config_default() {
    let config = ModelConfig::default();
    assert_eq!(config.provider, "anthropic");
    assert_eq!(config.model, "claude");
    assert_eq!(config.temperature, 0.7);
    assert_eq!(config.max_tokens, 4096);
}

#[tokio::test]
async fn test_permission_config_default() {
    let config = PermissionConfig::default();
    assert!(!config.allow_read);
    assert!(!config.allow_write);
    assert!(!config.allow_execute);
    assert!(config.allowed_paths.is_empty());
    assert!(config.denied_paths.is_empty());
}
