//! IPC Contract
//!
//! Inter-process communication contract for Knight-Agent.
//! Defines message types and protocols for communication between
//! Rust processes (TUI, Daemon, Session).
//!
//! Design Reference: docs/03-module-design/infrastructure/ipc-contract.md

// Re-export public API
pub use client::{ClientEvent, IpcClient, IpcClientConfig};
pub use codec::FrameCodec;
pub use contract::IPCContract;
pub use dispatch::MethodDispatcher;
pub use error::{ErrorCode, IPCError, IPCResult};
pub use implementation::{IPCConfig, IPCContractImpl};
pub use registry::{AwaitInfo, AwaitRegistry};
pub use server::{IpcServer, IpcServerConfig, ServerEvent, StreamingContext};
pub use transport::{Connection, TcpConnection, TcpTransport, Transport};
pub use types::{
    BaseMessage, ErrorResponse, MessageType, NotificationMessage, PendingQuery,
    QueryContext, QueryDependencies, QueryType, RequestMessage, RequestOptions,
    ResponseMessage, StreamChunkMessage, UserQueryMessage, UserResponseData,
    UserResponseMessage,
};

mod client;
mod codec;
mod contract;
mod dispatch;
mod error;
mod implementation;
mod registry;
mod server;
mod transport;
mod types;
