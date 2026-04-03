//! Storage Service
//!
//! Design Reference: docs/03-module-design/services/storage-service.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Storage not initialized")]
    NotInitialized,
    #[error("Storage read failed: {0}")]
    ReadFailed(String),
    #[error("Storage write failed: {0}")]
    WriteFailed(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: std::time::SystemTime,
}

#[async_trait]
pub trait StorageService: Send + Sync {
    fn new() -> Result<Self, StorageError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn put(&self, key: &str, value: serde_json::Value) -> Result<(), StorageError>;
    async fn get(&self, key: &str) -> Result<StorageEntry, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    async fn list_keys(&self) -> Result<Vec<String>, StorageError>;
}

pub struct StorageServiceImpl;

impl StorageService for StorageServiceImpl {
    fn new() -> Result<Self, StorageError> {
        Ok(StorageServiceImpl)
    }

    fn name(&self) -> &str {
        "storage-service"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn put(&self, key: &str, value: serde_json::Value) -> Result<(), StorageError> {
        let _ = key;
        let _ = value;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<StorageEntry, StorageError> {
        Err(StorageError::KeyNotFound(key.to_string()))
    }

    async fn delete(&self, _key: &str) -> Result<(), StorageError> {
        Ok(())
    }

    async fn list_keys(&self) -> Result<Vec<String>, StorageError> {
        Ok(vec![])
    }
}
