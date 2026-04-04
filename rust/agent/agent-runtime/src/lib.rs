//! Agent Runtime
//!
//! Design Reference: docs/03-module-design/agent/agent-runtime.md
//!
//! This module provides the core agent runtime system, including:
//! - Agent lifecycle management (create, start, stop, pause, resume)
//! - Message handling and context management
//! - State machine for agent status transitions
//! - Tool call proxy interface

pub mod types;
pub mod runtime;

pub use types::{
    AgentRuntimeError, AgentStatus, AgentState, AgentStatistics, Agent, AgentContext,
    ErrorInfo, AwaitInfo, Message, MessageRole, ContentBlock, ContentBlockType,
    ToolResult, MemoryItem, UserResponse, RuntimeResult,
};

pub use runtime::AgentRuntimeImpl;

/// Configuration for the agent runtime
pub use runtime::RuntimeConfig;
