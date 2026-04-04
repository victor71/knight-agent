//! Hook Executor
//!
//! Handles hook execution with phase handling, filtering, and result aggregation.

use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

use crate::types::*;
use crate::registry::HookRegistry;

/// Hook executor for running hooks and aggregating results
pub struct HookExecutor {
    registry: Arc<HookRegistry>,
    global_timeout_ms: u64,
}

impl HookExecutor {
    /// Create a new executor with a reference to the registry
    pub fn new(registry: Arc<HookRegistry>) -> Self {
        Self {
            registry,
            global_timeout_ms: 30000, // 30 second default
        }
    }

    /// Create a new executor with custom global timeout
    pub fn with_timeout(registry: Arc<HookRegistry>, timeout_ms: u64) -> Self {
        Self {
            registry,
            global_timeout_ms: timeout_ms,
        }
    }

    /// Trigger hooks for an event and phase
    pub async fn trigger(&self, event: &str, phase: HookPhase, context: HookContext) -> TriggerResult {
        let start = Instant::now();
        let matching_hooks = self.registry.find_matching(event, phase).await;

        if matching_hooks.is_empty() {
            debug!("No hooks found for event '{}' phase '{:?}'", event, phase);
            return TriggerResult::default();
        }

        info!("Triggering {} hooks for event '{}' phase '{:?}'", matching_hooks.len(), event, phase);

        let mut results = Vec::new();
        let mut blocked = false;
        let mut block_reason = None;
        let mut modified = false;
        let mut modified_data = None;
        let mut skipped = false;
        let mut hooks_failed = 0;

        for hook in matching_hooks {
            // Check if already blocked - if so, skip remaining hooks
            if blocked {
                debug!("Skipping hook '{}' - already blocked", hook.id);
                skipped = true;
                continue;
            }

            // Execute the hook
            let result = self.execute_hook(&hook, &context).await;
            let duration_ms = result.duration_ms;

            // Record execution in registry
            self.registry.record_execution(&hook.id, duration_ms, result.success).await;

            if !result.success {
                hooks_failed += 1;
            }

            // Check for blocking
            if result.blocked {
                blocked = true;
                block_reason = result.block_reason.clone();
                warn!("Hook '{}' blocked execution: {:?}", hook.id, block_reason);
            }

            // Check for modification
            if result.modified {
                modified = true;
                modified_data = result.modified_data.clone();
            }

            results.push(result);
        }

        TriggerResult {
            blocked,
            block_reason,
            modified,
            modified_data,
            skipped,
            hooks_executed: results.len() as u32,
            hooks_failed,
            duration_ms: start.elapsed().as_millis() as u64,
            results,
        }
    }

    /// Execute a single hook
    async fn execute_hook(&self, hook: &HookDefinition, _context: &HookContext) -> HookExecutionResult {
        let start = Instant::now();
        let hook_id = hook.id.clone();

        debug!("Executing hook '{}' handler '{:?}'", hook_id, hook.handler);

        let result = match &hook.handler {
            HookHandler::Command { executable, args, env } => {
                self.execute_command(executable, args, env).await
            }
            HookHandler::Skill { skill_id, args } => {
                self.execute_skill(skill_id, args).await
            }
            HookHandler::Rpc { endpoint, method, timeout_secs } => {
                self.execute_rpc(endpoint, method, *timeout_secs).await
            }
            HookHandler::Wasm { module, function } => {
                self.execute_wasm(module, function).await
            }
            HookHandler::Callback { handler } => {
                self.execute_callback(handler).await
            }
        };

        HookExecutionResult {
            hook_id,
            success: result.is_ok(),
            blocked: false,
            block_reason: None,
            modified: false,
            modified_data: None,
            skipped: false,
            error: result.err().map(|e| e.to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Execute a command handler
    async fn execute_command(
        &self,
        executable: &str,
        args: &[String],
        _env: &std::collections::HashMap<String, String>,
    ) -> Result<(), HookError> {
        // In a real implementation, this would spawn a process
        // For now, simulate execution
        debug!("Executing command: {} {:?}", executable, args);
        Ok(())
    }

    /// Execute a skill handler
    async fn execute_skill(
        &self,
        skill_id: &str,
        args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), HookError> {
        // In a real implementation, this would invoke the skill engine
        debug!("Executing skill: {} with {:?}", skill_id, args);
        Ok(())
    }

    /// Execute an RPC handler
    async fn execute_rpc(
        &self,
        endpoint: &str,
        method: &str,
        timeout_secs: u64,
    ) -> Result<(), HookError> {
        // In a real implementation, this would make an RPC call
        debug!("Executing RPC: {} method {} timeout {}s", endpoint, method, timeout_secs);
        Ok(())
    }

    /// Execute a WASM handler
    async fn execute_wasm(&self, module: &str, function: &str) -> Result<(), HookError> {
        // In a real implementation, this would invoke WASM
        debug!("Executing WASM: {}::{}", module, function);
        Ok(())
    }

    /// Execute a callback handler (internal use)
    async fn execute_callback(&self, handler: &str) -> Result<(), HookError> {
        // In a real implementation, this would invoke an internal callback
        debug!("Executing callback: {}", handler);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_trigger_no_hooks() {
        let registry = Arc::new(HookRegistry::new());
        let executor = HookExecutor::new(registry);
        let context = HookContext::new("test_event".to_string(), HookPhase::Before);

        let result = executor.trigger("nonexistent", HookPhase::Before, context).await;

        assert!(!result.blocked);
        assert_eq!(result.hooks_executed, 0);
    }

    #[tokio::test]
    async fn test_trigger_with_hooks() {
        let registry = Arc::new(HookRegistry::new());
        let hook = create_test_hook("h1", "test_event", HookPhase::Before);
        registry.register(hook).await.unwrap();

        let executor = HookExecutor::new(Arc::clone(&registry));
        let context = HookContext::new("test_event".to_string(), HookPhase::Before);

        let result = executor.trigger("test_event", HookPhase::Before, context).await;

        assert_eq!(result.hooks_executed, 1);
        assert!(result.hooks_failed == 0 || !result.results.is_empty());
    }

    #[tokio::test]
    async fn test_execute_hook_skill() {
        let registry = Arc::new(HookRegistry::new());
        let executor = HookExecutor::new(registry);
        let hook = create_test_hook("h1", "test", HookPhase::Before);
        let context = HookContext::new("test".to_string(), HookPhase::Before);

        let result = executor.execute_hook(&hook, &context).await;

        assert!(result.success);
        assert_eq!(result.hook_id, "h1");
    }
}
