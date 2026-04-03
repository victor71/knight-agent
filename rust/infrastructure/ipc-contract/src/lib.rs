//! IPC Contract
//!
//! Design Reference: docs/03-module-design/infrastructure/ipc-contract.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IPCError {
    #[error("IPC not initialized")]
    NotInitialized,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCMessage {
    pub id: String,
    pub msg_type: String,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait IPCContract: Send + Sync {
    fn new() -> Result<Self, IPCError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn send(&self, message: IPCMessage) -> Result<(), IPCError>;
    async fn receive(&self) -> Result<IPCMessage, IPCError>;
    async fn connect(&self) -> Result<(), IPCError>;
    async fn disconnect(&self) -> Result<(), IPCError>;
}

pub struct IPCContractImpl;

impl IPCContract for IPCContractImpl {
    fn new() -> Result<Self, IPCError> {
        Ok(IPCContractImpl)
    }

    fn name(&self) -> &str {
        "ipc-contract"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn send(&self, _message: IPCMessage) -> Result<(), IPCError> {
        Ok(())
    }

    async fn receive(&self) -> Result<IPCMessage, IPCError> {
        Err(IPCError::ReceiveFailed("No message available".to_string()))
    }

    async fn connect(&self) -> Result<(), IPCError> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), IPCError> {
        Ok(())
    }
}
