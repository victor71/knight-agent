//! Event Loop Tests

use event_loop::{
    Event, EventHandler, EventListener, EventLoopImpl, EventLoopTrait, EventSource,
    EventSourceType, HandlerType, SkillHandler,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_event_loop_lifecycle() {
    let loop_impl = EventLoopImpl::new().unwrap();
    assert_eq!(loop_impl.name(), "event-loop");
    assert!(!loop_impl.is_initialized());

    loop_impl.start().await.unwrap();
    assert!(loop_impl.is_initialized());

    loop_impl.stop(false).await.unwrap();
    assert!(!loop_impl.is_initialized());
}

#[tokio::test]
async fn test_register_source() {
    let loop_impl = EventLoopImpl::new().unwrap();
    let source = EventSource::new("src1", "Test Source", EventSourceType::FileWatcher);

    let source_id = loop_impl.register_source(source).await.unwrap();
    assert_eq!(source_id, "src1");

    let sources = loop_impl.list_sources().await.unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].id, "src1");
}

#[tokio::test]
async fn test_add_listener() {
    let loop_impl = EventLoopImpl::new().unwrap();
    let handler = EventHandler {
        handler_type: HandlerType::Skill,
        skill: Some(SkillHandler {
            skill_id: "test_skill".to_string(),
            args: HashMap::new(),
        }),
        hook: None,
        webhook: None,
    };
    let listener = EventListener::new("lst1", "Test Listener", handler);

    let listener_id = loop_impl.add_listener(listener).await.unwrap();
    assert_eq!(listener_id, "lst1");

    let listeners = loop_impl.list_listeners(None).await.unwrap();
    assert_eq!(listeners.len(), 1);
}

#[tokio::test]
async fn test_emit_event() {
    let loop_impl = EventLoopImpl::new().unwrap();
    let event = Event::new("e1", "test_event", "test");

    let count = loop_impl.emit(event).await.unwrap();
    assert_eq!(count, 1);

    let queue_info = loop_impl.get_queue_info().await.unwrap();
    assert_eq!(queue_info.size, 1);
}

#[tokio::test]
async fn test_emit_delayed() {
    let loop_impl = EventLoopImpl::new().unwrap();
    let event = Event::new("e1", "test_event", "test");

    let scheduled = loop_impl.emit_delayed(event, 1000).await.unwrap();
    assert!(scheduled);
    assert!(loop_impl.cancel_delayed("e1").await.unwrap());
}

#[tokio::test]
async fn test_get_status() {
    let loop_impl = EventLoopImpl::new().unwrap();
    loop_impl.start().await.unwrap();

    let status = loop_impl.get_status().unwrap();
    assert!(status.running);
    assert_eq!(status.uptime_seconds, 0);

    loop_impl.stop(false).await.unwrap();
}
