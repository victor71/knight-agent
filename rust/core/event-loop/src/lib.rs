//! Event Loop
//!
//! Design Reference: docs/03-module-design/core/event-loop.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EventLoopError {
    #[error("Event loop not initialized")]
    NotInitialized,
    #[error("Event processing failed: {0}")]
    ProcessingFailed(String),
    #[error("Shutdown requested")]
    ShutdownRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait EventLoop: Send + Sync {
    fn new() -> Result<Self, EventLoopError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn start(&self) -> Result<(), EventLoopError>;
    async fn stop(&self) -> Result<(), EventLoopError>;
    async fn dispatch(&self, event: Event) -> Result<(), EventLoopError>;
}

pub struct EventLoopImpl;

impl EventLoop for EventLoopImpl {
    fn new() -> Result<Self, EventLoopError> {
        Ok(EventLoopImpl)
    }

    fn name(&self) -> &str {
        "event-loop"
    }

    fn is_initialized(&self) -> bool {
        false // TODO: implement
    }

    async fn start(&self) -> Result<(), EventLoopError> {
        Ok(())
    }

    async fn stop(&self) -> Result<(), EventLoopError> {
        Ok(())
    }

    async fn dispatch(&self, _event: Event) -> Result<(), EventLoopError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_loop_lifecycle() {
        let loop_impl = EventLoopImpl::new().unwrap();
        assert_eq!(loop_impl.name(), "event-loop");
    }
}
