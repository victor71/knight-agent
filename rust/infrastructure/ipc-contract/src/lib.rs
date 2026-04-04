//! IPC Contract
//!
//! Inter-process communication contract for Knight-Agent.
//! Defines message types and protocols for communication between
//! Rust core services and TypeScript UI layer.
//!
//! Design Reference: docs/03-module-design/infrastructure/ipc-contract.md

// Re-export public API
pub use contract::IPCContract;
pub use error::{ErrorCode, IPCError, IPCResult};
pub use implementation::{IPCConfig, IPCContractImpl};
pub use registry::{AwaitInfo, AwaitRegistry};
pub use types::{
    BaseMessage, ErrorResponse, MessageType, NotificationMessage, PendingQuery,
    QueryContext, QueryDependencies, QueryType, RequestMessage, RequestOptions,
    ResponseMessage, StreamChunkMessage, UserQueryMessage, UserResponseData,
    UserResponseMessage,
};

mod contract;
mod error;
mod implementation;
mod registry;
mod types;
