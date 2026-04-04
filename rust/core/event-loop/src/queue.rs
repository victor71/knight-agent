//! Event Queue
//!
//! Thread-safe event queue with priority support.

use crate::types::{Event, OverflowPolicy, QueueInfo};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// A prioritized event with metadata for queue management
#[derive(Debug, Clone)]
struct PrioritizedEvent {
    event: Event,
    enqueued_at: Instant,
}

impl PartialEq for PrioritizedEvent {
    fn eq(&self, other: &Self) -> bool {
        // Compare by priority first, then by enqueued time for FIFO within same priority
        self.event.priority == other.event.priority
            && self.enqueued_at == other.enqueued_at
    }
}

impl Eq for PrioritizedEvent {}

impl PartialOrd for PrioritizedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // For BinaryHeap (max-heap): "larger" elements are popped first
        //
        // Priority: Lower number = higher priority, should be popped first
        //   To make 50 "larger" than 100 in max-heap: use other.priority.cmp(&self.priority)
        //   100.cmp(50) = Greater (other is "larger" = self with 50 is popped first) ✓
        //
        // FIFO: Earlier event should be popped first
        //   For max-heap to give FIFO, earlier (smaller timestamp) must be "larger"
        //   self.time.cmp(other.time) gives Less when self is earlier
        //   Less means "smaller" in max-heap... which gives LIFO not FIFO
        //
        //   To get FIFO from max-heap: reverse the timestamp comparison
        //   other.time.cmp(&self.time) gives Greater when self is earlier
        //   Greater means "larger" in max-heap = popped first ✓
        match other.event.priority.cmp(&self.event.priority) {
            Ordering::Equal => other.enqueued_at.cmp(&self.enqueued_at),
            ord => ord,
        }
    }
}

/// Thread-safe event queue with priority support
pub struct EventQueue {
    inner: Arc<RwLock<BinaryHeap<PrioritizedEvent>>>,
    capacity: usize,
    overflow_policy: OverflowPolicy,
}

impl EventQueue {
    /// Create a new event queue
    pub fn new(capacity: usize, overflow_policy: OverflowPolicy) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BinaryHeap::new())),
            capacity,
            overflow_policy,
        }
    }

    /// Push an event into the queue
    pub fn push(&self, event: Event) -> Result<(), QueueError> {
        let mut queue = self.inner.write().map_err(|_| QueueError::Poisoned)?;

        // Check capacity
        if queue.len() >= self.capacity {
            match self.overflow_policy {
                OverflowPolicy::Block => {
                    return Err(QueueError::QueueFull);
                }
                OverflowPolicy::DropOldest => {
                    // Remove the oldest (highest priority or earliest)
                    queue.pop();
                }
                OverflowPolicy::DropNewest => {
                    // Don't add the new event
                    return Err(QueueError::Dropped);
                }
            }
        }

        let prioritized = PrioritizedEvent {
            event,
            enqueued_at: Instant::now(),
        };
        queue.push(prioritized);
        Ok(())
    }

    /// Pop the highest priority event from the queue
    pub fn pop(&self) -> Option<Event> {
        let mut queue = self.inner.write().ok()?;
        queue.pop().map(|p| p.event)
    }

    /// Peek at the highest priority event without removing it
    pub fn peek(&self) -> Option<Event> {
        let queue = self.inner.read().ok()?;
        queue.peek().cloned().map(|p| p.event)
    }

    /// Get the current size of the queue
    pub fn len(&self) -> usize {
        self.inner.read().map(|q| q.len()).unwrap_or(0)
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get queue information
    pub fn info(&self) -> Option<QueueInfo> {
        let queue = self.inner.read().ok()?;
        let size = queue.len();

        let oldest_event_age_ms = queue
            .peek()
            .map(|p| p.enqueued_at.elapsed().as_millis() as u64)
            .unwrap_or(0);

        Some(QueueInfo {
            size,
            capacity: self.capacity,
            utilization_percent: (size as f64 / self.capacity as f64) * 100.0,
            oldest_event_age_ms,
        })
    }

    /// Clear all events from the queue
    pub fn clear(&self) {
        if let Ok(mut queue) = self.inner.write() {
            queue.clear();
        }
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new(10000, OverflowPolicy::Block)
    }
}

/// Queue errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum QueueError {
    #[error("Queue is full")]
    QueueFull,
    #[error("Event was dropped due to overflow policy")]
    Dropped,
    #[error("Queue lock is poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_event(id: &str, priority: u32) -> Event {
        let mut event = Event::new(id, "test", "test");
        event.priority = priority;
        event
    }

    #[test]
    fn test_queue_push_pop() {
        let queue = EventQueue::new(10, OverflowPolicy::Block);

        queue.push(create_event("1", 100)).unwrap();
        queue.push(create_event("2", 50)).unwrap(); // Higher priority
        queue.push(create_event("3", 100)).unwrap();

        // Should get priority 50 first
        let first = queue.pop().unwrap();
        assert_eq!(first.id, "2");
        assert_eq!(first.priority, 50);

        // Then FIFO for same priority (1 before 3)
        let second = queue.pop().unwrap();
        assert_eq!(second.id, "1");

        let third = queue.pop().unwrap();
        assert_eq!(third.id, "3");
    }

    #[test]
    fn test_queue_capacity() {
        let queue = EventQueue::new(2, OverflowPolicy::Block);

        queue.push(create_event("1", 100)).unwrap();
        queue.push(create_event("2", 100)).unwrap();

        // Queue is full, should fail
        assert!(matches!(queue.push(create_event("3", 100)), Err(QueueError::QueueFull)));
    }

    #[test]
    fn test_drop_newest_policy() {
        let queue = EventQueue::new(2, OverflowPolicy::DropNewest);

        queue.push(create_event("1", 100)).unwrap();
        queue.push(create_event("2", 100)).unwrap();

        // Should drop the new event
        assert!(matches!(queue.push(create_event("3", 100)), Err(QueueError::Dropped)));

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_drop_oldest_policy() {
        let queue = EventQueue::new(2, OverflowPolicy::DropOldest);

        queue.push(create_event("1", 100)).unwrap();
        queue.push(create_event("2", 100)).unwrap();

        // Should drop the oldest
        queue.push(create_event("3", 100)).unwrap();

        // Should contain 2 and 3, not 1
        let ids: Vec<_> = std::iter::from_fn(|| queue.pop()).collect();
        assert!(ids.iter().any(|e| e.id == "2"));
        assert!(ids.iter().any(|e| e.id == "3"));
        assert!(!ids.iter().any(|e| e.id == "1"));
    }

    #[test]
    fn test_queue_info() {
        let queue = EventQueue::new(100, OverflowPolicy::Block);

        queue.push(create_event("1", 100)).unwrap();
        queue.push(create_event("2", 100)).unwrap();

        let info = queue.info().unwrap();
        assert_eq!(info.size, 2);
        assert_eq!(info.capacity, 100);
        assert_eq!(info.utilization_percent, 2.0);
    }
}
