//! Hook Engine Tests
//!
//! Unit tests for the hook engine module.

use hook_engine::{
    HookContext, HookDefinition, HookExecutor, HookHandler, HookPhase, HookRegistry,
};
use std::collections::HashMap;

fn create_test_hook(id: &str, event: &str, phase: HookPhase) -> HookDefinition {
    HookDefinition::new(
        id.to_string(),
        event.to_string(),
        phase,
        HookHandler::Skill {
            skill_id: "test_skill".to_string(),
            args: HashMap::new(),
        },
    )
}

#[tokio::test]
async fn test_registry_register_and_get() {
    let registry = HookRegistry::new();
    let hook = create_test_hook("h1", "tool_call", HookPhase::Before);

    registry.register(hook).await.unwrap();
    let retrieved = registry.get("h1").await.unwrap();
    assert_eq!(retrieved.id, "h1");
    assert_eq!(retrieved.event, "tool_call");
}

#[tokio::test]
async fn test_registry_unregister() {
    let registry = HookRegistry::new();
    let hook = create_test_hook("h1", "tool_call", HookPhase::Before);

    registry.register(hook).await.unwrap();
    registry.unregister("h1").await.unwrap();
    assert!(registry.get("h1").await.is_err());
}

#[tokio::test]
async fn test_registry_register_duplicate() {
    let registry = HookRegistry::new();
    let hook = create_test_hook("h1", "tool_call", HookPhase::Before);

    registry.register(hook).await.unwrap();
    let result = registry
        .register(create_test_hook("h1", "tool_call", HookPhase::After))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_find_matching() {
    let registry = HookRegistry::new();

    registry
        .register(create_test_hook("h1", "tool_call", HookPhase::Before))
        .await
        .unwrap();
    registry
        .register(create_test_hook("h2", "tool_call", HookPhase::After))
        .await
        .unwrap();
    registry
        .register(create_test_hook("h3", "agent_execute", HookPhase::Before))
        .await
        .unwrap();

    let matching = registry.find_matching("tool_call", HookPhase::Before).await;
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].id, "h1");

    let matching_after = registry.find_matching("tool_call", HookPhase::After).await;
    assert_eq!(matching_after.len(), 1);
    assert_eq!(matching_after[0].id, "h2");
}

#[tokio::test]
async fn test_registry_enable_disable() {
    let registry = HookRegistry::new();
    let hook = create_test_hook("h1", "tool_call", HookPhase::Before);

    registry.register(hook).await.unwrap();

    // Before disable - should find the hook
    let matching = registry.find_matching("tool_call", HookPhase::Before).await;
    assert_eq!(matching.len(), 1);

    registry.disable("h1").await.unwrap();

    // After disable - should not find the hook
    let matching = registry.find_matching("tool_call", HookPhase::Before).await;
    assert!(matching.is_empty());

    registry.enable("h1").await.unwrap();

    // After enable - should find the hook again
    let matching = registry.find_matching("tool_call", HookPhase::Before).await;
    assert_eq!(matching.len(), 1);
}

#[tokio::test]
async fn test_registry_list() {
    let registry = HookRegistry::new();

    registry
        .register(create_test_hook("h1", "tool_call", HookPhase::Before))
        .await
        .unwrap();
    registry
        .register(create_test_hook("h2", "agent_execute", HookPhase::Before))
        .await
        .unwrap();

    let all = registry.list(None).await;
    assert_eq!(all.len(), 2);

    let filtered = registry.list(Some("tool_call")).await;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "h1");
}

#[tokio::test]
async fn test_registry_clear() {
    let registry = HookRegistry::new();

    registry
        .register(create_test_hook("h1", "tool_call", HookPhase::Before))
        .await
        .unwrap();
    registry
        .register(create_test_hook("h2", "tool_call", HookPhase::After))
        .await
        .unwrap();

    assert_eq!(registry.len().await, 2);

    registry.clear().await;

    assert!(registry.is_empty().await);
}

#[tokio::test]
async fn test_executor_trigger_no_hooks() {
    let registry = std::sync::Arc::new(HookRegistry::new());
    let executor = HookExecutor::new(std::sync::Arc::clone(&registry));
    let context = HookContext::new("nonexistent".to_string(), HookPhase::Before);

    let result = executor
        .trigger("nonexistent", HookPhase::Before, context)
        .await;

    assert!(!result.blocked);
    assert_eq!(result.hooks_executed, 0);
}

#[tokio::test]
async fn test_executor_trigger_with_hooks() {
    let registry = std::sync::Arc::new(HookRegistry::new());
    let hook = create_test_hook("h1", "test_event", HookPhase::Before);
    registry.register(hook).await.unwrap();

    let executor = HookExecutor::new(std::sync::Arc::clone(&registry));
    let context = HookContext::new("test_event".to_string(), HookPhase::Before);

    let result = executor
        .trigger("test_event", HookPhase::Before, context)
        .await;

    assert_eq!(result.hooks_executed, 1);
}

#[tokio::test]
async fn test_executor_records_execution() {
    let registry = std::sync::Arc::new(HookRegistry::new());
    let hook = create_test_hook("h1", "test_event", HookPhase::Before);
    registry.register(hook).await.unwrap();

    let executor = HookExecutor::new(std::sync::Arc::clone(&registry));
    let context = HookContext::new("test_event".to_string(), HookPhase::Before);

    executor
        .trigger("test_event", HookPhase::Before, context)
        .await;

    // Check that execution was recorded
    let hook_info = registry.list(None).await;
    assert_eq!(hook_info.len(), 1);
    assert_eq!(hook_info[0].execution_count, 1);
}

#[tokio::test]
async fn test_hook_definition_new() {
    let handler = HookHandler::Skill {
        skill_id: "test_skill".to_string(),
        args: HashMap::new(),
    };
    let hook = HookDefinition::new(
        "h1".to_string(),
        "tool_call".to_string(),
        HookPhase::Before,
        handler,
    );

    assert_eq!(hook.id, "h1");
    assert_eq!(hook.event, "tool_call");
    assert_eq!(hook.phase, HookPhase::Before);
    assert!(hook.enabled);
    assert_eq!(hook.priority, 100);
}

#[tokio::test]
async fn test_hook_context_builder() {
    let ctx = HookContext::new("agent_execute".to_string(), HookPhase::Before)
        .with_data("key", serde_json::json!("value"));

    assert_eq!(ctx.event, "agent_execute");
    assert_eq!(ctx.phase, HookPhase::Before);
    assert!(ctx.data.contains_key("key"));
}

#[tokio::test]
async fn test_hook_execution_result_success() {
    let result = hook_engine::HookExecutionResult::success("h1".to_string());
    assert!(result.success);
    assert!(!result.blocked);
}

#[tokio::test]
async fn test_hook_execution_result_blocked() {
    let result =
        hook_engine::HookExecutionResult::blocked("h1".to_string(), "Access denied".to_string());
    assert!(!result.success);
    assert!(result.blocked);
    assert_eq!(result.block_reason, Some("Access denied".to_string()));
}
