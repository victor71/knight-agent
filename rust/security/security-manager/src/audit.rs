//! Audit Logger
//!
//! Handles security event logging and querying.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::types::*;

/// Audit logger for security events
pub struct AuditLogger {
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    config: AuditConfig,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Log a security event
    pub async fn log_event(&self, event: SecurityEvent) -> String {
        let event_id = event.id.clone();

        // Store event
        let mut events = self.events.write().await;

        // Check retention limit
        let max_events = (self.config.retention_days as usize) * 1000; // Approximate limit
        if events.len() >= max_events {
            // Remove oldest 10%
            let remove_count = max_events / 10;
            events.drain(0..remove_count);
        }

        events.push(event);
        event_id
    }

    /// Query events based on filter
    pub async fn query(&self, query: LogQuery) -> Vec<SecurityEvent> {
        let events = self.events.read().await;

        events
            .iter()
            .filter(|event| {
                // Filter by time range
                if let Some(ref time_range) = query.time_range {
                    if let Some(end) = time_range.end {
                        if event.timestamp < time_range.start || event.timestamp > end {
                            return false;
                        }
                    } else if event.timestamp < time_range.start {
                        return false;
                    }
                }

                // Filter by event types
                if let Some(ref types) = query.event_types {
                    if !types.contains(&event.event_type) {
                        return false;
                    }
                }

                // Filter by principal
                if let Some(ref principal) = query.principal {
                    if &event.principal != principal {
                        return false;
                    }
                }

                // Filter by resource
                if let Some(ref resource) = query.resource {
                    if event.resource.as_ref() != Some(resource) {
                        return false;
                    }
                }

                true
            })
            .skip(query.offset.unwrap_or(0))
            .take(query.limit.unwrap_or(100))
            .cloned()
            .collect()
    }

    /// Get log summary statistics
    pub async fn get_summary(&self, time_range: Option<TimeRange>) -> LogSummary {
        let events = self.events.read().await;

        let filtered: Vec<_> = events
            .iter()
            .filter(|event| {
                if let Some(ref range) = time_range {
                    if let Some(end) = range.end {
                        if event.timestamp < range.start || event.timestamp > end {
                            return false;
                        }
                    } else if event.timestamp < range.start {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total_events = filtered.len();
        let mut by_event_type: HashMap<String, usize> = HashMap::new();
        let mut by_principal: HashMap<String, usize> = HashMap::new();
        let mut denied_count = 0;
        let mut threat_count = 0;

        for event in filtered {
            *by_event_type.entry(format!("{:?}", event.event_type)).or_insert(0) += 1;
            *by_principal.entry(event.principal.to_string()).or_insert(0) += 1;

            if matches!(event.result, EventResult::Denied) {
                denied_count += 1;
            }

            if matches!(event.event_type, SecurityEventType::ThreatDetected) {
                threat_count += 1;
            }
        }

        LogSummary {
            total_events,
            by_event_type,
            by_principal,
            denied_count,
            threat_count,
        }
    }

    /// Clear all events
    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        events.clear();
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(AuditConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_event() {
        let logger = AuditLogger::default();

        let event = SecurityEvent {
            id: "event-1".to_string(),
            timestamp: std::time::SystemTime::now(),
            event_type: SecurityEventType::AccessRequest,
            principal: Principal::User("test-user".to_string()),
            resource: Some("resource:1".to_string()),
            action: Some("read".to_string()),
            result: EventResult::Allowed,
            reason: None,
            details: HashMap::new(),
        };

        let event_id = logger.log_event(event.clone()).await;
        assert_eq!(event_id, "event-1");

        let query = LogQuery::default();
        let events = logger.query(query).await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_query_by_principal() {
        let logger = AuditLogger::default();

        logger
            .log_event(SecurityEvent {
                id: "1".to_string(),
                timestamp: std::time::SystemTime::now(),
                event_type: SecurityEventType::AccessRequest,
                principal: Principal::User("user1".to_string()),
                resource: None,
                action: None,
                result: EventResult::Allowed,
                reason: None,
                details: HashMap::new(),
            })
            .await;

        logger
            .log_event(SecurityEvent {
                id: "2".to_string(),
                timestamp: std::time::SystemTime::now(),
                event_type: SecurityEventType::AccessRequest,
                principal: Principal::User("user2".to_string()),
                resource: None,
                action: None,
                result: EventResult::Allowed,
                reason: None,
                details: HashMap::new(),
            })
            .await;

        let query = LogQuery {
            principal: Some(Principal::User("user1".to_string())),
            ..Default::default()
        };

        let events = logger.query(query).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "1");
    }
}
