//! Event Loop Implementation
//!
//! Main event loop implementation with queue management and async operations.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::dispatcher::EventDispatcher;
use crate::queue::EventQueue;
use crate::scheduler::EventScheduler;
use crate::types::*;

/// Event loop errors
#[derive(Error, Debug)]
pub enum EventLoopError {
    #[error("Event loop not initialized")]
    NotInitialized,
    #[error("Event processing failed: {0}")]
    ProcessingFailed(String),
    #[error("Shutdown requested")]
    ShutdownRequested,
    #[error("Source not found: {0}")]
    SourceNotFound(String),
    #[error("Listener not found: {0}")]
    ListenerNotFound(String),
    #[error("Event not found: {0}")]
    EventNotFound(String),
    #[error("Queue error: {0}")]
    QueueError(String),
}

/// Result type for event loop operations
pub type EventLoopResult<T> = Result<T, EventLoopError>;

/// Event loop trait
#[async_trait]
pub trait EventLoopTrait: Send + Sync {
    /// Create a new event loop instance
    fn new() -> EventLoopResult<Self>
    where
        Self: Sized;

    /// Get the event loop name
    fn name(&self) -> &str;

    /// Check if the event loop is initialized and running
    fn is_initialized(&self) -> bool;

    /// Start the event loop
    async fn start(&self) -> EventLoopResult<()>;

    /// Stop the event loop
    async fn stop(&self, graceful: bool) -> EventLoopResult<()>;

    /// Get event loop status
    fn get_status(&self) -> EventLoopResult<EventLoopStatus>;

    // ========== Event Source Management ==========

    /// Register an event source
    async fn register_source(&self, source: EventSource) -> EventLoopResult<String>;

    /// Unregister an event source
    async fn unregister_source(&self, source_id: &str) -> EventLoopResult<()>;

    /// Enable an event source
    async fn enable_source(&self, source_id: &str) -> EventLoopResult<()>;

    /// Disable an event source
    async fn disable_source(&self, source_id: &str) -> EventLoopResult<()>;

    /// List all event sources
    async fn list_sources(&self) -> EventLoopResult<Vec<EventSourceInfo>>;

    // ========== Listener Management ==========

    /// Add an event listener
    async fn add_listener(&self, listener: EventListener) -> EventLoopResult<String>;

    /// Remove an event listener
    async fn remove_listener(&self, listener_id: &str) -> EventLoopResult<()>;

    /// List all listeners (optionally filtered by event type)
    async fn list_listeners(
        &self,
        event_type: Option<&str>,
    ) -> EventLoopResult<Vec<EventListenerInfo>>;

    // ========== Event Operations ==========

    /// Emit an event (synchronous dispatch)
    async fn emit(&self, event: Event) -> EventLoopResult<usize>;

    /// Emit an event with delay
    async fn emit_delayed(&self, event: Event, delay_ms: u64) -> EventLoopResult<bool>;

    /// Cancel a delayed event
    async fn cancel_delayed(&self, event_id: &str) -> EventLoopResult<bool>;

    /// Dispatch an event to matching listeners (internal)
    async fn dispatch(&self, event: Event) -> EventLoopResult<()>;

    // ========== Statistics ==========

    /// Get event statistics
    async fn get_stats(&self) -> EventLoopResult<EventStats>;

    /// Get queue information
    async fn get_queue_info(&self) -> EventLoopResult<QueueInfo>;
}

/// Event loop implementation
pub struct EventLoopImpl {
    #[allow(dead_code)]
    config: EventLoopConfig,
    queue: Arc<EventQueue>,
    dispatcher: Arc<RwLock<EventDispatcher>>,
    scheduler: Arc<EventScheduler>,
    sources: Arc<RwLock<HashMap<String, EventSource>>>,
    running: Arc<RwLock<bool>>,
    started_at: Arc<RwLock<Option<Instant>>>,
    stats: Arc<RwLock<EventStats>>,
}

impl EventLoopImpl {
    /// Create a new event loop with default configuration
    pub fn with_config(config: EventLoopConfig) -> EventLoopResult<Self> {
        Ok(Self {
            config: config.clone(),
            queue: Arc::new(EventQueue::new(config.queue_size, config.overflow_policy)),
            dispatcher: Arc::new(RwLock::new(EventDispatcher::new())),
            scheduler: Arc::new(EventScheduler::new()),
            sources: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            started_at: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(EventStats {
                total_events: 0,
                events_by_type: HashMap::new(),
                events_by_source: HashMap::new(),
                processing_time_avg_ms: 0.0,
                error_count: 0,
            })),
        })
    }

    /// Record an event in statistics
    #[allow(dead_code)]
    async fn record_event(&self, event: &Event, processing_time_ms: f64, delivered_count: usize) {
        // Use try_write to avoid blocking
        if let Ok(mut guard) = self.stats.try_write() {
            guard.total_events += 1;

            // Update events by type
            *guard
                .events_by_type
                .entry(event.event_type.clone())
                .or_insert(0) += 1;

            // Update events by source
            *guard
                .events_by_source
                .entry(event.source.clone())
                .or_insert(0) += 1;

            // Update average processing time
            let n = guard.total_events as f64;
            guard.processing_time_avg_ms =
                (guard.processing_time_avg_ms * (n - 1.0) + processing_time_ms) / n;

            debug!(
                "Event '{}' processed in {:.2}ms, delivered to {} listeners",
                event.id, processing_time_ms, delivered_count
            );
        }
    }
}

impl Default for EventLoopImpl {
    fn default() -> Self {
        Self::with_config(EventLoopConfig::default()).unwrap()
    }
}

#[async_trait]
impl EventLoopTrait for EventLoopImpl {
    fn new() -> EventLoopResult<Self> {
        Self::with_config(EventLoopConfig::default())
    }

    fn name(&self) -> &str {
        "event-loop"
    }

    fn is_initialized(&self) -> bool {
        // Check if running guard can be acquired
        self.running.try_read().map(|r| *r).unwrap_or(false)
    }

    async fn start(&self) -> EventLoopResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        *self.started_at.write().await = Some(Instant::now());

        info!("Event loop started");
        Ok(())
    }

    async fn stop(&self, graceful: bool) -> EventLoopResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping event loop (graceful={})", graceful);

        if graceful {
            // Wait for queue to drain (with timeout)
            let timeout = tokio::time::Duration::from_secs(5);
            let start = Instant::now();
            while !self.queue.is_empty() && start.elapsed() < timeout {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        *running = false;

        info!("Event loop stopped");
        Ok(())
    }

    fn get_status(&self) -> EventLoopResult<EventLoopStatus> {
        let running = self.running.try_read().map(|g| *g).unwrap_or(false);
        let uptime_seconds = self
            .started_at
            .try_read()
            .ok()
            .and_then(|guard| *guard)
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);

        let sources_len = self.sources.try_read().map(|g| g.len()).unwrap_or(0);
        let listeners_len = self.dispatcher.try_read().map(|g| g.len()).unwrap_or(0);

        let stats = self
            .stats
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| EventStats {
                total_events: 0,
                events_by_type: HashMap::new(),
                events_by_source: HashMap::new(),
                processing_time_avg_ms: 0.0,
                error_count: 0,
            });

        Ok(EventLoopStatus {
            running,
            uptime_seconds,
            events_processed: stats.total_events,
            events_per_second: if uptime_seconds > 0 {
                stats.total_events as f64 / uptime_seconds as f64
            } else {
                0.0
            },
            active_sources: sources_len,
            active_listeners: listeners_len,
        })
    }

    async fn register_source(&self, source: EventSource) -> EventLoopResult<String> {
        let source_id = source.id.clone();
        let mut sources = self.sources.write().await;
        sources.insert(source_id.clone(), source);
        info!("Registered event source: {}", source_id);
        Ok(source_id)
    }

    async fn unregister_source(&self, source_id: &str) -> EventLoopResult<()> {
        let mut sources = self.sources.write().await;
        if sources.remove(source_id).is_some() {
            info!("Unregistered event source: {}", source_id);
            Ok(())
        } else {
            Err(EventLoopError::SourceNotFound(source_id.to_string()))
        }
    }

    async fn enable_source(&self, source_id: &str) -> EventLoopResult<()> {
        let mut sources = self.sources.write().await;
        if let Some(source) = sources.get_mut(source_id) {
            source.enabled = true;
            info!("Enabled event source: {}", source_id);
            Ok(())
        } else {
            Err(EventLoopError::SourceNotFound(source_id.to_string()))
        }
    }

    async fn disable_source(&self, source_id: &str) -> EventLoopResult<()> {
        let mut sources = self.sources.write().await;
        if let Some(source) = sources.get_mut(source_id) {
            source.enabled = false;
            info!("Disabled event source: {}", source_id);
            Ok(())
        } else {
            Err(EventLoopError::SourceNotFound(source_id.to_string()))
        }
    }

    async fn list_sources(&self) -> EventLoopResult<Vec<EventSourceInfo>> {
        let sources = self.sources.read().await;
        Ok(sources.values().map(EventSourceInfo::from).collect())
    }

    async fn add_listener(&self, listener: EventListener) -> EventLoopResult<String> {
        let listener_id = listener.id.clone();
        let mut dispatcher = self.dispatcher.write().await;
        dispatcher.add_listener(listener);
        info!("Added event listener: {}", listener_id);
        Ok(listener_id)
    }

    async fn remove_listener(&self, listener_id: &str) -> EventLoopResult<()> {
        let mut dispatcher = self.dispatcher.write().await;
        if dispatcher.remove_listener(listener_id) {
            info!("Removed event listener: {}", listener_id);
            Ok(())
        } else {
            Err(EventLoopError::ListenerNotFound(listener_id.to_string()))
        }
    }

    async fn list_listeners(
        &self,
        event_type: Option<&str>,
    ) -> EventLoopResult<Vec<EventListenerInfo>> {
        let dispatcher = self.dispatcher.read().await;
        let listeners = dispatcher.list_listeners();

        let filtered: Vec<EventListenerInfo> = if let Some(et) = event_type {
            listeners
                .iter()
                .filter(|l| {
                    l.filter
                        .event_type
                        .as_ref()
                        .map(|f| matches!(f, serde_json::Value::String(s) if s == et || s == "*"))
                        .unwrap_or(true)
                })
                .map(EventListenerInfo::from)
                .collect()
        } else {
            listeners.iter().map(EventListenerInfo::from).collect()
        };

        Ok(filtered)
    }

    async fn emit(&self, event: Event) -> EventLoopResult<usize> {
        // Add to queue
        self.queue
            .push(event.clone())
            .map_err(|e| EventLoopError::QueueError(e.to_string()))?;

        debug!("Emitted event: {} ({})", event.id, event.event_type);
        Ok(1)
    }

    async fn emit_delayed(&self, event: Event, delay_ms: u64) -> EventLoopResult<bool> {
        Ok(self.scheduler.schedule(event, delay_ms))
    }

    async fn cancel_delayed(&self, event_id: &str) -> EventLoopResult<bool> {
        Ok(self.scheduler.cancel(event_id))
    }

    async fn dispatch(&self, event: Event) -> EventLoopResult<()> {
        let results = {
            let dispatcher = self.dispatcher.read().await;
            dispatcher.dispatch(&event).await
        };

        let success_count = results.iter().filter(|r| r.success).count();
        let error_count = results.iter().filter(|r| !r.success).count();

        if error_count > 0 {
            warn!(
                "Event {} dispatched with {} successes and {} errors",
                event.id, success_count, error_count
            );
        }

        Ok(())
    }

    async fn get_stats(&self) -> EventLoopResult<EventStats> {
        let stats = self.stats.read().await.clone();
        Ok(stats)
    }

    async fn get_queue_info(&self) -> EventLoopResult<QueueInfo> {
        self.queue
            .info()
            .ok_or_else(|| EventLoopError::ProcessingFailed("Failed to get queue info".to_string()))
    }
}
