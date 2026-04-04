//! Timer Scheduler Tests

use timer_system::scheduler::TimerScheduler;
use timer_system::{Timer, TimerCallback, TimerStatus};
use std::collections::HashMap;

#[tokio::test]
async fn test_add_and_get_timer() {
    let scheduler = TimerScheduler::new();
    let timer = Timer::oneshot(
        "test1".to_string(),
        1000,
        TimerCallback::Skill {
            skill_id: "test".to_string(),
            args: HashMap::new(),
        },
    );

    scheduler.add_oneshot(timer.clone()).await.unwrap();

    let retrieved = scheduler.get_timer("test1").await.unwrap();
    assert_eq!(retrieved.id, "test1");
}

#[tokio::test]
async fn test_cancel_timer() {
    let scheduler = TimerScheduler::new();
    let timer = Timer::oneshot(
        "test1".to_string(),
        1000,
        TimerCallback::Skill {
            skill_id: "test".to_string(),
            args: HashMap::new(),
        },
    );

    scheduler.add_oneshot(timer).await.unwrap();
    scheduler.cancel("test1").await.unwrap();

    let result = scheduler.get_timer("test1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pause_resume() {
    let scheduler = TimerScheduler::new();
    let timer = Timer::interval(
        "test1".to_string(),
        1000,
        TimerCallback::Skill {
            skill_id: "test".to_string(),
            args: HashMap::new(),
        },
    );

    scheduler.add_interval(timer).await.unwrap();
    scheduler.pause("test1").await.unwrap();

    let timer = scheduler.get_timer("test1").await.unwrap();
    assert_eq!(timer.status, TimerStatus::Paused);

    scheduler.resume("test1").await.unwrap();
    let timer = scheduler.get_timer("test1").await.unwrap();
    assert_eq!(timer.status, TimerStatus::Active);
}

#[tokio::test]
async fn test_list_timers() {
    let scheduler = TimerScheduler::new();

    let timer1 = Timer::oneshot("t1".to_string(), 1000, TimerCallback::Callback { handler: String::new() });
    let timer2 = Timer::interval("t2".to_string(), 1000, TimerCallback::Callback { handler: String::new() });

    scheduler.add_oneshot(timer1).await.unwrap();
    scheduler.add_interval(timer2).await.unwrap();

    let timers = scheduler.list_timers().await;
    assert_eq!(timers.len(), 2);
}

#[tokio::test]
async fn test_stats() {
    let scheduler = TimerScheduler::new();

    let timer1 = Timer::oneshot("t1".to_string(), 1000, TimerCallback::Callback { handler: String::new() });
    let timer2 = Timer::interval("t2".to_string(), 1000, TimerCallback::Callback { handler: String::new() });

    scheduler.add_oneshot(timer1).await.unwrap();
    scheduler.add_interval(timer2).await.unwrap();

    let stats = scheduler.get_stats().await;
    assert_eq!(stats.total_timers, 2);
    assert_eq!(stats.active_timers, 2);
}
