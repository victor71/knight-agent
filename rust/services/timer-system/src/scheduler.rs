//! Timer Scheduler
//!
//! Handles timer scheduling and execution.

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info};

use crate::types::*;

/// A scheduled timer entry with ordering for the priority queue
#[derive(Debug, Clone)]
struct TimerEntry {
    timer: Timer,
    scheduled_at: Instant,
    next_execution: Instant,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.next_execution == other.next_execution && self.timer.id == other.timer.id
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // For BinaryHeap (max-heap), "larger" elements are popped first
        // We want timers with earliest next_execution to be popped first
        // So earlier time = "larger" in ordering
        match self.next_execution.cmp(&other.next_execution) {
            Ordering::Equal => other.scheduled_at.cmp(&self.scheduled_at),
            ord => ord.reverse(),
        }
    }
}

/// Timer scheduler implementation
pub struct TimerScheduler {
    timers: Arc<AsyncRwLock<HashMap<String, Timer>>>,
    schedule: Arc<AsyncRwLock<BinaryHeap<TimerEntry>>>,
    stats: Arc<AsyncRwLock<TimerStats>>,
    running: Arc<AsyncRwLock<bool>>,
}

impl TimerScheduler {
    /// Create a new timer scheduler
    pub fn new() -> Self {
        Self {
            timers: Arc::new(AsyncRwLock::new(HashMap::new())),
            schedule: Arc::new(AsyncRwLock::new(BinaryHeap::new())),
            stats: Arc::new(AsyncRwLock::new(TimerStats::default())),
            running: Arc::new(AsyncRwLock::new(false)),
        }
    }

    /// Check if the scheduler is running
    pub fn is_running(&self) -> bool {
        self.running.try_read().map(|g| *g).unwrap_or(false)
    }

    /// Start the scheduler
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        *running = true;
        info!("Timer scheduler started");
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Timer scheduler stopped");
    }

    /// Add a oneshot timer
    pub async fn add_oneshot(&self, timer: Timer) -> Result<String, TimerError> {
        if timer.status != TimerStatus::Pending {
            return Err(TimerError::CreationFailed("Timer must be in pending status".into()));
        }

        let timer_id = timer.id.clone();
        self.add_timer(timer).await?;
        Ok(timer_id)
    }

    /// Add an interval timer
    pub async fn add_interval(&self, mut timer: Timer) -> Result<String, TimerError> {
        if timer.timer_type != TimerType::Interval {
            return Err(TimerError::CreationFailed("Timer type must be interval".into()));
        }

        let timer_id = timer.id.clone();

        // Calculate first execution time
        if let TimerConfig::Interval(config) = &mut timer.config {
            let next = chrono::Utc::now() + chrono::Duration::milliseconds(config.interval_ms as i64);
            config.next_execution = Some(next.to_rfc3339());
        }

        timer.status = TimerStatus::Active;
        self.add_timer(timer).await?;
        Ok(timer_id)
    }

    /// Add a cron timer
    pub async fn add_cron(&self, mut timer: Timer) -> Result<String, TimerError> {
        if timer.timer_type != TimerType::Cron {
            return Err(TimerError::CreationFailed("Timer type must be cron".into()));
        }

        let timer_id = timer.id.clone();

        // Calculate next execution from cron expression
        if let TimerConfig::Cron(config) = &mut timer.config {
            if let Some(next) = self.parse_cron_next(&config.expression) {
                config.next_execution = Some(next);
            }
        }

        timer.status = TimerStatus::Active;
        self.add_timer(timer).await?;
        Ok(timer_id)
    }

    /// Internal method to add a timer to the scheduler
    async fn add_timer(&self, timer: Timer) -> Result<(), TimerError> {
        let timer_id = timer.id.clone();

        // Insert into timers map
        {
            let mut timers = self.timers.write().await;
            timers.insert(timer_id.clone(), timer.clone());
        }

        // Calculate next execution time
        let next_execution = match &timer.config {
            TimerConfig::Oneshot(config) => {
                let created = chrono::DateTime::parse_from_rfc3339(&timer.created_at)
                    .map_err(|e| TimerError::CreationFailed(e.to_string()))?;
                let next = created + chrono::Duration::milliseconds(config.delay_ms as i64);
                Instant::now() + Duration::from_millis(config.delay_ms)
            }
            TimerConfig::Interval(config) => {
                if let Some(next_str) = &config.next_execution {
                    chrono::DateTime::parse_from_rfc3339(next_str)
                        .map(|_| Instant::now() + Duration::from_millis(config.interval_ms))
                        .unwrap_or_else(|_| Instant::now() + Duration::from_millis(config.interval_ms))
                } else {
                    Instant::now() + Duration::from_millis(config.interval_ms)
                }
            }
            TimerConfig::Cron(_) => {
                // For cron, use a default - actual calculation would use a cron parser
                Instant::now() + Duration::from_secs(60)
            }
        };

        // Add to schedule
        let entry = TimerEntry {
            timer,
            scheduled_at: Instant::now(),
            next_execution,
        };

        {
            let mut schedule = self.schedule.write().await;
            schedule.push(entry);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_timers += 1;
            stats.active_timers += 1;
        }

        info!("Added timer: {}", timer_id);
        Ok(())
    }

    /// Cancel a timer
    pub async fn cancel(&self, timer_id: &str) -> Result<(), TimerError> {
        let mut timers = self.timers.write().await;
        if timers.remove(timer_id).is_some() {
            // Update stats
            drop(timers);
            {
                let mut stats = self.stats.write().await;
                if stats.active_timers > 0 {
                    stats.active_timers -= 1;
                }
                stats.completed_timers += 1;
            }
            info!("Cancelled timer: {}", timer_id);
            Ok(())
        } else {
            Err(TimerError::NotFound(timer_id.to_string()))
        }
    }

    /// Pause a timer
    pub async fn pause(&self, timer_id: &str) -> Result<(), TimerError> {
        let mut timers = self.timers.write().await;
        if let Some(timer) = timers.get_mut(timer_id) {
            timer.status = TimerStatus::Paused;
            timer.updated_at = chrono::Utc::now().to_rfc3339();

            // Update stats
            drop(timers);
            {
                let mut stats = self.stats.write().await;
                if stats.active_timers > 0 {
                    stats.active_timers -= 1;
                }
                stats.paused_timers += 1;
            }

            info!("Paused timer: {}", timer_id);
            Ok(())
        } else {
            Err(TimerError::NotFound(timer_id.to_string()))
        }
    }

    /// Resume a paused timer
    pub async fn resume(&self, timer_id: &str) -> Result<(), TimerError> {
        let mut timers = self.timers.write().await;
        if let Some(timer) = timers.get_mut(timer_id) {
            if timer.status != TimerStatus::Paused {
                return Err(TimerError::CreationFailed("Timer is not paused".into()));
            }

            timer.status = TimerStatus::Active;
            timer.updated_at = chrono::Utc::now().to_rfc3339();

            // Update stats
            drop(timers);
            {
                let mut stats = self.stats.write().await;
                if stats.paused_timers > 0 {
                    stats.paused_timers -= 1;
                }
                stats.active_timers += 1;
            }

            info!("Resumed timer: {}", timer_id);
            Ok(())
        } else {
            Err(TimerError::NotFound(timer_id.to_string()))
        }
    }

    /// Get a timer by ID
    pub async fn get_timer(&self, timer_id: &str) -> Result<Timer, TimerError> {
        let timers = self.timers.read().await;
        timers
            .get(timer_id)
            .cloned()
            .ok_or_else(|| TimerError::NotFound(timer_id.to_string()))
    }

    /// List all timers
    pub async fn list_timers(&self) -> Vec<Timer> {
        let timers = self.timers.read().await;
        timers.values().cloned().collect()
    }

    /// Get timer statistics
    pub async fn get_stats(&self) -> TimerStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get timers that are ready to execute
    pub async fn get_ready_timers(&self) -> Vec<Timer> {
        let mut ready = Vec::new();
        let mut schedule = self.schedule.write().await;
        let now = Instant::now();

        // Find all timers that are ready
        let to_remove: Vec<_> = schedule
            .iter()
            .filter(|entry| entry.next_execution <= now)
            .map(|entry| entry.timer.id.clone())
            .collect();

        // Remove ready timers from schedule and collect them
        for id in &to_remove {
            if let Some(entry) = schedule.iter().find(|e| e.timer.id == *id) {
                ready.push(entry.timer.clone());
            }
        }

        // Remove from schedule
        schedule.retain(|entry| entry.next_execution > now);

        ready
    }

    /// Execute a timer and reschedule if needed
    pub async fn execute_timer(&self, timer_id: &str) -> Result<TimerExecutionResult, TimerError> {
        let mut timers = self.timers.write().await;
        let timer = timers
            .get_mut(timer_id)
            .ok_or_else(|| TimerError::NotFound(timer_id.to_string()))?;

        let start = Instant::now();
        let scheduled_at = timer.last_execution.clone().unwrap_or_else(|| timer.created_at.clone());

        // Execute the callback (for now, just simulate success)
        // In a real implementation, this would trigger the callback
        let result = TimerExecutionResult {
            status: ExecutionStatus::Success,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
            output: None,
        };

        // Update timer
        timer.last_execution = Some(chrono::Utc::now().to_rfc3339());
        timer.last_result = Some(result.clone());
        timer.updated_at = chrono::Utc::now().to_rfc3339();

        // Handle based on timer type
        match &mut timer.config {
            TimerConfig::Oneshot(_) => {
                timer.status = TimerStatus::Completed;
                if let Some(mut t) = timers.remove(timer_id) {
                    t.status = TimerStatus::Completed;
                    timers.insert(timer_id.to_string(), t);
                }
            }
            TimerConfig::Interval(config) => {
                config.execution_count += 1;
                if config.max_executions > 0 && config.execution_count >= config.max_executions as u32 {
                    timer.status = TimerStatus::Completed;
                } else {
                    // Reschedule
                    let next = chrono::Utc::now() + chrono::Duration::milliseconds(config.interval_ms as i64);
                    config.next_execution = Some(next.to_rfc3339());
                }
            }
            TimerConfig::Cron(config) => {
                // Calculate next cron execution
                if let Some(next) = self.parse_cron_next(&config.expression) {
                    config.next_execution = Some(next);
                }
            }
        }

        // Update stats
        drop(timers);
        {
            let mut stats = self.stats.write().await;
            stats.total_executions += 1;
            match result.status {
                ExecutionStatus::Success => stats.successful_executions += 1,
                ExecutionStatus::Failed | ExecutionStatus::Timeout => stats.failed_executions += 1,
            }

            // Update average execution time
            let n = stats.total_executions as f64;
            stats.avg_execution_time_ms =
                (stats.avg_execution_time_ms * (n - 1.0) + result.duration_ms as f64) / n;
        }

        debug!("Executed timer {} in {:?}, result: {:?}", timer_id, start.elapsed(), result.status);
        Ok(result)
    }

    /// Parse cron expression and calculate next execution
    /// Simplified implementation - a full cron parser would be more complete
    fn parse_cron_next(&self, expression: &str) -> Option<String> {
        // Simplified cron parser for basic expressions
        // Format: "minute hour day month weekday" or "second minute hour day month weekday"
        let parts: Vec<&str> = expression.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let now = chrono::Utc::now();
        let next = match parts.len() {
            5 => {
                // Standard cron (minute hour day month weekday)
                // Very simplified - just adds 1 minute for now
                now + chrono::Duration::minutes(1)
            }
            6 => {
                // Cron with seconds
                // Simplified - adds 1 minute
                now + chrono::Duration::minutes(1)
            }
            _ => return None,
        };

        Some(next.to_rfc3339())
    }

    /// Reset a timer (restart its countdown)
    pub async fn reset(&self, timer_id: &str) -> Result<(), TimerError> {
        let mut timers = self.timers.write().await;
        if let Some(timer) = timers.get_mut(timer_id) {
            timer.updated_at = chrono::Utc::now().to_rfc3339();

            match &mut timer.config {
                TimerConfig::Oneshot(config) => {
                    // Reset to original delay
                    let created = chrono::DateTime::parse_from_rfc3339(&timer.created_at)
                        .map_err(|e| TimerError::CreationFailed(e.to_string()))?;
                    let next = created + chrono::Duration::milliseconds(config.delay_ms as i64);
                    config.execute_at = Some(next.to_rfc3339());
                }
                TimerConfig::Interval(config) => {
                    config.execution_count = 0;
                    let next = chrono::Utc::now() + chrono::Duration::milliseconds(config.interval_ms as i64);
                    config.next_execution = Some(next.to_rfc3339());
                }
                TimerConfig::Cron(config) => {
                    if let Some(next) = self.parse_cron_next(&config.expression) {
                        config.next_execution = Some(next);
                    }
                }
            }

            timer.status = TimerStatus::Active;
            info!("Reset timer: {}", timer_id);
            Ok(())
        } else {
            Err(TimerError::NotFound(timer_id.to_string()))
        }
    }

    /// Clear all timers
    pub async fn clear(&self) {
        let mut timers = self.timers.write().await;
        let mut schedule = self.schedule.write().await;
        let mut stats = self.stats.write().await;

        timers.clear();
        schedule.clear();
        *stats = TimerStats::default();

        info!("Cleared all timers");
    }
}

impl Default for TimerScheduler {
    fn default() -> Self {
        Self::new()
    }
}
