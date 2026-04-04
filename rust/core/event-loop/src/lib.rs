//! Event Loop
//!
//! Event-driven architecture for the Knight Agent system.
//! Handles event sources, listeners, queue management, and event dispatching.
//!
//! Design Reference: docs/03-module-design/core/event-loop.md

mod dispatcher;
mod event_loop;
mod queue;
mod scheduler;
mod types;

pub use dispatcher::{DispatchResult, EventDispatcher};
pub use event_loop::{EventLoopError, EventLoopImpl, EventLoopResult, EventLoopTrait};
pub use queue::{EventQueue, QueueError};
pub use scheduler::EventScheduler;
pub use types::{EventLoopStatus, *};

