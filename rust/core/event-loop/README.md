# Event Loop Module

Event-driven architecture for the Knight Agent system. Handles event sources, listeners, queue management, and event dispatching.

## Design Reference

See [docs/03-module-design/core/event-loop.md](../../../../docs/03-module-design/core/event-loop.md) for high-level design.

## Public API

### Core Types

```rust
// Event loop status
pub struct EventLoopStatus {
    pub running: bool,
    pub uptime_seconds: u64,
    pub events_processed: u64,
    pub events_per_second: f64,
    pub active_sources: usize,
    pub active_listeners: usize,
}

// Event queue information
pub struct QueueInfo {
    pub size: usize,
    pub capacity: usize,
    pub utilization_percent: f64,
    pub oldest_event_age_ms: u64,
}

// Event statistics
pub struct EventStats {
    pub total_events: u64,
    pub events_by_type: HashMap<String, u64>,
    pub events_by_source: HashMap<String, u64>,
    pub processing_time_avg_ms: f64,
    pub error_count: u64,
}
```

### Event Loop Trait

```rust
pub trait EventLoopTrait: Send + Sync {
    // Lifecycle
    fn new() -> EventLoopResult<Self>;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn start(&self) -> EventLoopResult<()>;
    async fn stop(&self, graceful: bool) -> EventLoopResult<()>;
    fn get_status(&self) -> EventLoopResult<EventLoopStatus>;

    // Event Source Management
    async fn register_source(&self, source: EventSource) -> EventLoopResult<String>;
    async fn unregister_source(&self, source_id: &str) -> EventLoopResult<()>;
    async fn enable_source(&self, source_id: &str) -> EventLoopResult<()>;
    async fn disable_source(&self, source_id: &str) -> EventLoopResult<()>;
    async fn list_sources(&self) -> EventLoopResult<Vec<EventSourceInfo>>;

    // Listener Management
    async fn add_listener(&self, listener: EventListener) -> EventLoopResult<String>;
    async fn remove_listener(&self, listener_id: &str) -> EventLoopResult<()>;
    async fn list_listeners(&self, event_type: Option<&str>) -> EventLoopResult<Vec<EventListenerInfo>>;

    // Event Operations
    async fn emit(&self, event: Event) -> EventLoopResult<usize>;
    async fn emit_delayed(&self, event: Event, delay_ms: u64) -> EventLoopResult<bool>;
    async fn cancel_delayed(&self, event_id: &str) -> EventLoopResult<bool>;
    async fn dispatch(&self, event: Event) -> EventLoopResult<()>;

    // Statistics
    async fn get_stats(&self) -> EventLoopResult<EventStats>;
    async fn get_queue_info(&self) -> EventLoopResult<QueueInfo>;
}
```

### Event Sources

```rust
// Source types
pub enum EventSourceType {
    FileWatcher,
    GitWatcher,
    Custom,
    Timer,
}

// Create a file watcher source
let source = EventSource::new("src1", "File Watcher", EventSourceType::FileWatcher);
```

### Event Listeners

```rust
use crate::types::{EventHandler, HandlerType, SkillHandler};

// Create a skill-based listener
let handler = EventHandler {
    handler_type: HandlerType::Skill,
    skill: Some(SkillHandler {
        skill_id: "my_skill".to_string(),
        args: HashMap::new(),
    }),
    hook: None,
    webhook: None,
};

let listener = EventListener::new("lst1", "My Listener", handler);
```

### Events

```rust
use crate::types::Event;

// Create a simple event
let event = Event::new("evt1", "file_change", "file_watcher");

// Create an event with data
let event = Event::with_data(
    "evt2",
    "git_commit",
    "git_watcher",
    serde_json::json!({ "branch": "main", "files": ["a.rs", "b.rs"] }),
);
```

## Usage Example

```rust
use event_loop::{EventLoopImpl, EventSource, EventSourceType, Event, EventListener, EventHandler, HandlerType, SkillHandler};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create event loop
    let loop_impl = EventLoopImpl::new()?;

    // Start the loop
    loop_impl.start().await?;

    // Register an event source
    let source = EventSource::new("files", "File Watcher", EventSourceType::FileWatcher);
    loop_impl.register_source(source).await?;

    // Add a listener
    let handler = EventHandler {
        handler_type: HandlerType::Skill,
        skill: Some(SkillHandler {
            skill_id: "process_file".to_string(),
            args: HashMap::new(),
        }),
        hook: None,
        webhook: None,
    };
    let listener = EventListener::new("lst1", "File Handler", handler);
    loop_impl.add_listener(listener).await?;

    // Emit events
    let event = Event::new("evt1", "file_change", "files");
    loop_impl.emit(event).await?;

    // Get status
    let status = loop_impl.get_status()?;
    println!("Running: {}, Events: {}", status.running, status.events_processed);

    // Stop gracefully
    loop_impl.stop(true).await?;

    Ok(())
}
```

## Queue Configuration

The event queue supports priority-based ordering and configurable overflow policies:

```rust
use event_loop::{EventLoopConfig, OverflowPolicy};

let config = EventLoopConfig {
    queue_size: 10000,
    overflow_policy: OverflowPolicy::DropOldest, // or Block, DropNewest
    workers: 4,
    batch_size: 10,
    metrics_enabled: true,
};

let loop_impl = EventLoopImpl::with_config(config)?;
```

### Overflow Policies

- **Block**: Reject new events when queue is full (default)
- **DropOldest**: Remove the oldest event to make room for new ones
- **DropNewest**: Drop the newly submitted event when queue is full

## Module Structure

```
event-loop/
├── src/
│   ├── lib.rs          # Module exports
│   ├── types.rs        # Core types (Event, EventSource, EventListener, etc.)
│   ├── queue.rs        # Priority queue implementation
│   ├── scheduler.rs    # Delayed event scheduling
│   ├── dispatcher.rs   # Event dispatch to listeners
│   └── event_loop.rs  # Main event loop implementation
├── Cargo.toml
└── README.md
```

## Dependencies

- `async-trait`: For async trait methods
- `tokio`: Async runtime with RwLock
- `tracing`: Structured logging
- `serde`: Serialization
- `chrono`: Timestamp handling
- `thiserror`: Error handling
