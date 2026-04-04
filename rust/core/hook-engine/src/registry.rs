//! Hook Registry
//!
//! Hook registration, lookup, and management.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::types::*;

/// Hook registry for managing hook definitions
pub struct HookRegistry {
    hooks: Arc<RwLock<HashMap<String, HookDefinition>>>,
    hooks_by_event: Arc<RwLock<HashMap<String, Vec<String>>>>, // event -> hook_ids
    execution_stats: Arc<RwLock<HashMap<String, HookStats>>>,
}

#[derive(Debug, Clone, Default)]
struct HookStats {
    execution_count: u64,
    last_executed: Option<String>,
    total_duration_ms: u64,
}

impl HookRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(HashMap::new())),
            hooks_by_event: Arc::new(RwLock::new(HashMap::new())),
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a hook
    pub async fn register(&self, hook: HookDefinition) -> Result<String, HookError> {
        let hook_id = hook.id.clone();
        let event = hook.event.clone();

        // Check if hook already exists
        {
            let hooks = self.hooks.read().await;
            if hooks.contains_key(&hook_id) {
                return Err(HookError::AlreadyExists(hook_id));
            }
        }

        // Insert into hooks map
        {
            let mut hooks = self.hooks.write().await;
            hooks.insert(hook_id.clone(), hook);
        }

        // Add to event index
        {
            let mut by_event = self.hooks_by_event.write().await;
            by_event
                .entry(event)
                .or_insert_with(Vec::new)
                .push(hook_id.clone());
        }

        info!("Registered hook: {}", hook_id);
        Ok(hook_id)
    }

    /// Unregister a hook
    pub async fn unregister(&self, hook_id: &str) -> Result<(), HookError> {
        // Remove from hooks map
        let hook = {
            let mut hooks = self.hooks.write().await;
            hooks.remove(hook_id).ok_or_else(|| HookError::NotFound(hook_id.to_string()))?
        };

        // Remove from event index
        {
            let mut by_event = self.hooks_by_event.write().await;
            if let Some(ids) = by_event.get_mut(&hook.event) {
                ids.retain(|id| id != hook_id);
            }
        }

        // Remove stats
        {
            let mut stats = self.execution_stats.write().await;
            stats.remove(hook_id);
        }

        info!("Unregistered hook: {}", hook_id);
        Ok(())
    }

    /// Get a hook by ID
    pub async fn get(&self, hook_id: &str) -> Result<HookDefinition, HookError> {
        let hooks = self.hooks.read().await;
        hooks
            .get(hook_id)
            .cloned()
            .ok_or_else(|| HookError::NotFound(hook_id.to_string()))
    }

    /// List all hooks, optionally filtered by event
    pub async fn list(&self, event_filter: Option<&str>) -> Vec<HookInfo> {
        let hooks = self.hooks.read().await;
        let stats = self.execution_stats.read().await;

        hooks
            .values()
            .filter(|hook| {
                if let Some(evt) = event_filter {
                    &hook.event == evt
                } else {
                    true
                }
            })
            .map(|hook| {
                let mut info = HookInfo::from(hook);
                if let Some(stat) = stats.get(&hook.id) {
                    info.execution_count = stat.execution_count;
                    info.last_executed = stat.last_executed.clone();
                }
                info
            })
            .collect()
    }

    /// Find hooks matching an event and phase
    pub async fn find_matching(&self, event: &str, phase: HookPhase) -> Vec<HookDefinition> {
        let by_event = self.hooks_by_event.read().await;
        let hooks = self.hooks.read().await;

        let mut matching: Vec<_> = by_event
            .get(event)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| hooks.get(id))
                    .filter(|hook| hook.phase == phase && hook.enabled)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        // Sort by priority (lower number = higher priority)
        matching.sort_by(|a, b| a.priority.cmp(&b.priority));
        matching
    }

    /// Enable a hook
    pub async fn enable(&self, hook_id: &str) -> Result<(), HookError> {
        let mut hooks = self.hooks.write().await;
        if let Some(hook) = hooks.get_mut(hook_id) {
            hook.enabled = true;
            info!("Enabled hook: {}", hook_id);
            Ok(())
        } else {
            Err(HookError::NotFound(hook_id.to_string()))
        }
    }

    /// Disable a hook
    pub async fn disable(&self, hook_id: &str) -> Result<(), HookError> {
        let mut hooks = self.hooks.write().await;
        if let Some(hook) = hooks.get_mut(hook_id) {
            hook.enabled = false;
            info!("Disabled hook: {}", hook_id);
            Ok(())
        } else {
            Err(HookError::NotFound(hook_id.to_string()))
        }
    }

    /// Record hook execution
    pub async fn record_execution(&self, hook_id: &str, duration_ms: u64, _success: bool) {
        let mut stats = self.execution_stats.write().await;
        let stat = stats.entry(hook_id.to_string()).or_default();
        stat.execution_count += 1;
        stat.last_executed = Some(chrono::Utc::now().to_rfc3339());
        stat.total_duration_ms += duration_ms;
    }

    /// Check if a hook exists
    pub async fn exists(&self, hook_id: &str) -> bool {
        let hooks = self.hooks.read().await;
        hooks.contains_key(hook_id)
    }

    /// Get count of hooks
    pub async fn len(&self) -> usize {
        let hooks = self.hooks.read().await;
        hooks.len()
    }

    /// Check if empty
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Clear all hooks
    pub async fn clear(&self) {
        let mut hooks = self.hooks.write().await;
        let mut by_event = self.hooks_by_event.write().await;
        let mut stats = self.execution_stats.write().await;

        hooks.clear();
        by_event.clear();
        stats.clear();

        info!("Cleared all hooks");
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_hook(id: &str, event: &str, phase: HookPhase) -> HookDefinition {
        HookDefinition::new(
            id.to_string(),
            event.to_string(),
            phase,
            HookHandler::Skill {
                skill_id: "test".to_string(),
                args: HashMap::new(),
            },
        )
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = HookRegistry::new();
        let hook = create_test_hook("h1", "tool_call", HookPhase::Before);

        registry.register(hook).await.unwrap();
        let retrieved = registry.get("h1").await.unwrap();
        assert_eq!(retrieved.id, "h1");
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = HookRegistry::new();
        let hook = create_test_hook("h1", "tool_call", HookPhase::Before);

        registry.register(hook).await.unwrap();
        registry.unregister("h1").await.unwrap();
        assert!(registry.get("h1").await.is_err());
    }

    #[tokio::test]
    async fn test_find_matching() {
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
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let registry = HookRegistry::new();
        let hook = create_test_hook("h1", "tool_call", HookPhase::Before);

        registry.register(hook).await.unwrap();
        registry.disable("h1").await.unwrap();

        let matching = registry.find_matching("tool_call", HookPhase::Before).await;
        assert!(matching.is_empty());

        registry.enable("h1").await.unwrap();
        let matching = registry.find_matching("tool_call", HookPhase::Before).await;
        assert_eq!(matching.len(), 1);
    }

    #[tokio::test]
    async fn test_list_with_filter() {
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
    }
}
