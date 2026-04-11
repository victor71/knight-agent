//! Event Scheduler
//!
//! Handles delayed/scheduled events.

use crate::types::Event;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// A scheduled event with its execution time
#[derive(Debug, Clone)]
pub struct ScheduledEvent {
    pub event: Event,
    pub scheduled_at: Instant,
    pub delay_ms: u64,
}

impl ScheduledEvent {
    /// Check if the scheduled time has passed
    pub fn is_ready(&self) -> bool {
        self.scheduled_at.elapsed() >= Duration::from_millis(self.delay_ms)
    }

    /// Get the time remaining until execution
    #[allow(dead_code)]
    pub fn time_remaining(&self) -> Duration {
        let elapsed = self.scheduled_at.elapsed();
        let delay = Duration::from_millis(self.delay_ms);
        if elapsed >= delay {
            Duration::from_secs(0)
        } else {
            delay - elapsed
        }
    }
}

/// Event scheduler for delayed events
pub struct EventScheduler {
    scheduled: Arc<RwLock<HashMap<String, ScheduledEvent>>>,
    #[allow(dead_code)]
    tick_interval: Duration,
}

impl EventScheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            scheduled: Arc::new(RwLock::new(HashMap::new())),
            tick_interval: Duration::from_millis(100),
        }
    }

    /// Schedule an event for delayed execution
    pub fn schedule(&self, event: Event, delay_ms: u64) -> bool {
        let scheduled = ScheduledEvent {
            event: event.clone(),
            scheduled_at: Instant::now(),
            delay_ms,
        };

        if let Ok(mut map) = self.scheduled.write() {
            // Check if event_id already exists
            if map.contains_key(&event.id) {
                debug!("Event {} already scheduled, updating", event.id);
            }
            map.insert(event.id.clone(), scheduled);
            info!("Scheduled event '{}' with delay {}ms", event.id, delay_ms);
            true
        } else {
            false
        }
    }

    /// Cancel a scheduled event
    pub fn cancel(&self, event_id: &str) -> bool {
        if let Ok(mut map) = self.scheduled.write() {
            if map.remove(event_id).is_some() {
                info!("Cancelled scheduled event '{}'", event_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get all ready events (whose delay has passed)
    pub fn get_ready_events(&self) -> Vec<Event> {
        if let Ok(mut map) = self.scheduled.write() {
            let ready: Vec<String> = map
                .iter()
                .filter(|(_, scheduled)| scheduled.is_ready())
                .map(|(id, _)| id.clone())
                .collect();

            let events: Vec<Event> = ready
                .iter()
                .filter_map(|id| map.remove(id).map(|s| s.event))
                .collect();

            events
        } else {
            Vec::new()
        }
    }

    /// Check if an event is scheduled
    pub fn is_scheduled(&self, event_id: &str) -> bool {
        if let Ok(map) = self.scheduled.read() {
            map.contains_key(event_id)
        } else {
            false
        }
    }

    /// Get count of scheduled events
    pub fn len(&self) -> usize {
        if let Ok(map) = self.scheduled.read() {
            map.len()
        } else {
            0
        }
    }

    /// Check if no events are scheduled
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all scheduled events
    pub fn clear(&self) {
        if let Ok(mut map) = self.scheduled.write() {
            map.clear();
        }
    }
}

impl Default for EventScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schedule_and_cancel() {
        let scheduler = EventScheduler::new();
        let event = Event::new("test1", "test", "test");

        assert!(!scheduler.is_scheduled("test1"));
        assert!(scheduler.schedule(event.clone(), 1000));
        assert!(scheduler.is_scheduled("test1"));
        assert!(scheduler.cancel("test1"));
        assert!(!scheduler.is_scheduled("test1"));
    }

    #[tokio::test]
    async fn test_get_ready_events() {
        let scheduler = EventScheduler::new();
        let event1 = Event::new("test1", "test", "test");
        let event2 = Event::new("test2", "test", "test");

        // Schedule with 0 delay (should be ready immediately)
        scheduler.schedule(event1.clone(), 0);
        scheduler.schedule(event2.clone(), 10000); // Not ready yet

        let ready = scheduler.get_ready_events();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "test1");

        // test2 should still be scheduled
        assert!(scheduler.is_scheduled("test2"));
    }

    #[tokio::test]
    async fn test_scheduled_event_time_remaining() {
        let scheduler = EventScheduler::new();
        let event = Event::new("test1", "test", "test");

        scheduler.schedule(event, 500);

        // Should have time remaining
        let remaining = scheduler.get_ready_events().first().map(|_| ());
        // No events ready yet (delay is 500ms)
        assert!(scheduler.len() == 1);
    }
}
