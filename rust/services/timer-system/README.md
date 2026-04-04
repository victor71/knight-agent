# Timer System Module

Manages all timed tasks and scheduling for Knight-Agent. Supports one-shot, interval, and cron timers.

## Design Reference

See [docs/03-module-design/services/timer-system.md](../../../../docs/03-module-design/services/timer-system.md) for detailed design.

## Public API

### Core Types

```rust
// Timer types
pub enum TimerType {
    Oneshot,   // One-time timer
    Interval,  // Repeating timer with fixed interval
    Cron,      // Cron-based scheduling
}

// Timer status
pub enum TimerStatus {
    Pending,
    Active,
    Paused,
    Completed,
    Cancelled,
}

// Timer callback types
pub enum TimerCallback {
    Callback { handler: String },
    Hook { hook_id: String, args: HashMap<String, JsonValue> },
    Skill { skill_id: String, args: HashMap<String, JsonValue> },
    Webhook { url: String, method: String, headers: HashMap, body: JsonValue },
}

// Timer statistics
pub struct TimerStats {
    pub total_timers: u64,
    pub active_timers: u64,
    pub paused_timers: u64,
    pub completed_timers: u64,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_time_ms: f64,
}
```

### Timer Creation

```rust
use timer_system::{Timer, TimerCallback, TimerScheduler};
use std::collections::HashMap;

// Create a one-shot timer (fires once after delay)
let timer = Timer::oneshot(
    "my_timer".to_string(),
    5000,  // 5 seconds delay
    TimerCallback::Skill {
        skill_id: "my_skill".to_string(),
        args: HashMap::new(),
    },
);

// Create an interval timer (repeats at fixed intervals)
let timer = Timer::interval(
    "repeat_timer".to_string(),
    60000,  // 1 minute interval
    TimerCallback::Hook {
        hook_id: "my_hook".to_string(),
        args: HashMap::new(),
    },
);

// Create a cron timer
let timer = Timer::cron(
    "cron_timer".to_string(),
    "0 9 * * *".to_string(),  // Every day at 9 AM
    TimerCallback::Webhook {
        url: "https://example.com/webhook".to_string(),
        method: "POST".to_string(),
        headers: HashMap::new(),
        body: serde_json::json!({}),
    },
);
```

### Scheduler API

```rust
use timer_system::scheduler::TimerScheduler;

let scheduler = TimerScheduler::new();

// Start the scheduler
scheduler.start().await;

// Add timers
scheduler.add_oneshot(timer).await?;
scheduler.add_interval(timer).await?;
scheduler.add_cron(timer).await?;

// Control timers
scheduler.pause("timer_id").await?;
scheduler.resume("timer_id").await?;
scheduler.cancel("timer_id").await?;
scheduler.reset("timer_id").await?;

// Query timers
let timer = scheduler.get_timer("timer_id").await?;
let all_timers = scheduler.list_timers().await;
let stats = scheduler.get_stats().await;

// Get timers ready to fire
let ready = scheduler.get_ready_timers().await;

// Execute a timer
let result = scheduler.execute_timer("timer_id").await?;
```

### Timer Filtering

```rust
use timer_system::types::TimerFilter;

// Filter timers
let filter = TimerFilter {
    timer_type: Some(TimerType::Interval),
    status: Some(TimerStatus::Active),
    name_pattern: Some("backup_*".to_string()),
    persistent: Some(true),
    ..Default::default()
};
```

## Usage Example

```rust
use timer_system::{Timer, TimerCallback, scheduler::TimerScheduler};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = TimerScheduler::new();
    scheduler.start().await?;

    // Create a timer that sends a daily report
    let report_timer = Timer::cron(
        "daily_report".to_string(),
        "0 8 * * *".to_string(),
        TimerCallback::Skill {
            skill_id: "send_daily_report".to_string(),
            args: HashMap::new(),
        },
    );

    scheduler.add_cron(report_timer).await?;
    println!("Daily report timer created");

    // Create a reminder for 30 minutes from now
    let reminder = Timer::oneshot(
        "meeting_reminder".to_string(),
        30 * 60 * 1000,  // 30 minutes
        TimerCallback::Webhook {
            url: "https://example.com/notify".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: serde_json::json!({ "message": "Meeting starting soon!" }),
        },
    );

    scheduler.add_oneshot(reminder).await?;
    println!("Meeting reminder timer created");

    // Query stats
    let stats = scheduler.get_stats().await;
    println!("Active timers: {}", stats.active_timers);

    Ok(())
}
```

## Module Structure

```
timer-system/
├── src/
│   ├── lib.rs          # Module exports
│   ├── types.rs        # Core types (Timer, TimerCallback, TimerStats, etc.)
│   └── scheduler.rs   # Timer scheduler implementation
├── Cargo.toml
└── README.md
```

## Dependencies

- `tokio`: Async runtime
- `serde`: Serialization
- `chrono`: Date/time handling
- `thiserror`: Error handling
- `tracing`: Structured logging
