//! Event Dispatcher
//!
//! Event dispatch logic including listener matching and handler execution.

use crate::types::{Event, EventListener, HandlerType};
use tracing::{debug, info};

/// Event dispatcher result
#[derive(Debug)]
pub struct DispatchResult {
    pub listener_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Dispatcher for matching events to listeners and executing handlers
pub struct EventDispatcher {
    listeners: Vec<EventListener>,
}

impl EventDispatcher {
    /// Create a new dispatcher
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    /// Add a listener
    pub fn add_listener(&mut self, listener: EventListener) {
        self.listeners.push(listener);
    }

    /// Remove a listener by ID
    pub fn remove_listener(&mut self, listener_id: &str) -> bool {
        if let Some(pos) = self.listeners.iter().position(|l| l.id == listener_id) {
            self.listeners.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get listener by ID
    pub fn get_listener(&self, listener_id: &str) -> Option<&EventListener> {
        self.listeners.iter().find(|l| l.id == listener_id)
    }

    /// List all listeners
    pub fn list_listeners(&self) -> &[EventListener] {
        &self.listeners
    }

    /// Count listeners
    pub fn len(&self) -> usize {
        self.listeners.len()
    }

    /// Check if dispatcher is empty
    pub fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }

    /// Find matching listeners for an event
    pub fn find_matching_listeners(&self, event: &Event) -> Vec<&EventListener> {
        self.listeners
            .iter()
            .filter(|listener| self.matches_filter(listener, event))
            .collect()
    }

    /// Check if an event matches a listener's filter
    fn matches_filter(&self, listener: &EventListener, event: &Event) -> bool {
        if !listener.enabled {
            return false;
        }

        // Check event type filter
        if let Some(ref event_type_filter) = listener.filter.event_type {
            if !self.matches_event_type(event_type_filter, &event.event_type) {
                return false;
            }
        }

        // Check source filter
        if let Some(ref source_filter) = listener.filter.source {
            if !self.matches_event_type(source_filter, &event.source) {
                return false;
            }
        }

        // Check custom conditions
        for (key, expected) in &listener.filter.conditions {
            if let Some(actual) = event.data.get(key) {
                if actual != expected {
                    return false;
                }
            } else if let Some(actual) = event.metadata.get(key) {
                if actual != expected {
                    return false;
                }
            }
        }

        true
    }

    /// Match event type or source against filter
    fn matches_event_type(&self, filter: &serde_json::Value, value: &str) -> bool {
        match filter {
            serde_json::Value::String(s) => s == value || s == "*",
            serde_json::Value::Array(arr) => arr.iter().any(|v| {
                if let Some(s) = v.as_str() {
                    s == value || s == "*"
                } else {
                    false
                }
            }),
            _ => false,
        }
    }

    /// Dispatch an event to all matching listeners
    pub async fn dispatch(&self, event: &Event) -> Vec<DispatchResult> {
        let listeners = self.find_matching_listeners(event);
        let mut results = Vec::with_capacity(listeners.len());

        for listener in listeners {
            let result = self.execute_handler(listener, event).await;
            results.push(result);
        }

        results
    }

    /// Execute a listener's handler
    async fn execute_handler(&self, listener: &EventListener, event: &Event) -> DispatchResult {
        debug!(
            "Executing handler for listener '{}' on event '{}'",
            listener.id, event.id
        );

        match &listener.handler.handler_type {
            HandlerType::Skill => {
                if let Some(ref skill) = listener.handler.skill {
                    self.handle_skill(listener, skill, event).await
                } else {
                    DispatchResult {
                        listener_id: listener.id.clone(),
                        success: false,
                        error: Some("Skill handler missing skill config".to_string()),
                    }
                }
            }
            HandlerType::Hook => {
                if let Some(ref hook) = listener.handler.hook {
                    self.handle_hook(listener, hook, event).await
                } else {
                    DispatchResult {
                        listener_id: listener.id.clone(),
                        success: false,
                        error: Some("Hook handler missing hook config".to_string()),
                    }
                }
            }
            HandlerType::Webhook => {
                if let Some(ref webhook) = listener.handler.webhook {
                    self.handle_webhook(listener, webhook, event).await
                } else {
                    DispatchResult {
                        listener_id: listener.id.clone(),
                        success: false,
                        error: Some("Webhook handler missing webhook config".to_string()),
                    }
                }
            }
            HandlerType::Callback => {
                // For callback, we just log - actual callback would need function reference
                info!(
                    "Callback handler triggered for listener '{}' on event '{}'",
                    listener.id, event.id
                );
                DispatchResult {
                    listener_id: listener.id.clone(),
                    success: true,
                    error: None,
                }
            }
        }
    }

    /// Handle skill trigger
    async fn handle_skill(
        &self,
        listener: &EventListener,
        skill: &crate::types::SkillHandler,
        event: &Event,
    ) -> DispatchResult {
        info!(
            "Triggering skill '{}' for listener '{}', event '{}'",
            skill.skill_id, listener.id, event.id
        );
        // TODO: Integrate with Skill Engine when available
        // For now, just log success
        DispatchResult {
            listener_id: listener.id.clone(),
            success: true,
            error: None,
        }
    }

    /// Handle hook trigger
    async fn handle_hook(
        &self,
        listener: &EventListener,
        hook: &crate::types::HookHandler,
        event: &Event,
    ) -> DispatchResult {
        info!(
            "Triggering hook '{}' for listener '{}', event '{}'",
            hook.hook_id, listener.id, event.id
        );
        // TODO: Integrate with Hook Engine when available
        // For now, just log success
        DispatchResult {
            listener_id: listener.id.clone(),
            success: true,
            error: None,
        }
    }

    /// Handle webhook call
    async fn handle_webhook(
        &self,
        listener: &EventListener,
        webhook: &crate::types::WebhookHandler,
        event: &Event,
    ) -> DispatchResult {
        info!(
            "Calling webhook '{}' for listener '{}', event '{}'",
            webhook.url, listener.id, event.id
        );
        // TODO: Make actual HTTP call when webhooks are needed
        // For now, just log success
        DispatchResult {
            listener_id: listener.id.clone(),
            success: true,
            error: None,
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EventFilter, EventHandler, EventListener, SkillHandler};

    fn create_test_listener(id: &str, event_type: &str) -> EventListener {
        EventListener {
            id: id.to_string(),
            name: id.to_string(),
            enabled: true,
            filter: EventFilter {
                event_type: Some(serde_json::json!(event_type)),
                source: None,
                conditions: std::collections::HashMap::new(),
            },
            handler: EventHandler {
                handler_type: HandlerType::Skill,
                skill: Some(SkillHandler {
                    skill_id: "test_skill".to_string(),
                    args: std::collections::HashMap::new(),
                }),
                hook: None,
                webhook: None,
            },
            error_handling: Default::default(),
        }
    }

    #[test]
    fn test_find_matching_listeners() {
        let dispatcher = EventDispatcher::new();
        let event = Event::new("e1", "file_change", "file_watcher");

        // Empty dispatcher
        assert!(dispatcher.find_matching_listeners(&event).is_empty());

        // Add listener matching "file_change"
        let mut dispatcher = EventDispatcher::new();
        dispatcher.add_listener(create_test_listener("l1", "file_change"));
        let matches = dispatcher.find_matching_listeners(&event);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "l1");

        // Add listener for different event type
        dispatcher.add_listener(create_test_listener("l2", "git_commit"));
        let matches = dispatcher.find_matching_listeners(&event);
        assert_eq!(matches.len(), 1);

        // Wildcard listener
        dispatcher.add_listener(create_test_listener("l3", "*"));
        let matches = dispatcher.find_matching_listeners(&event);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_remove_listener() {
        let mut dispatcher = EventDispatcher::new();
        dispatcher.add_listener(create_test_listener("l1", "file_change"));

        assert_eq!(dispatcher.len(), 1);
        assert!(dispatcher.remove_listener("l1"));
        assert_eq!(dispatcher.len(), 0);
        assert!(!dispatcher.remove_listener("nonexistent"));
    }
}
