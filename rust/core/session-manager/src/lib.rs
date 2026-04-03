//! Session Manager
//!
//! Design Reference: docs/03-module-design/core/session-manager.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session already exists: {0}")]
    AlreadyExists(String),
    #[error("Session expired")]
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: std::time::SystemTime,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait SessionManager: Send + Sync {
    fn new() -> Result<Self, SessionError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn create_session(&self, id: String) -> Result<Session, SessionError>;
    async fn get_session(&self, id: &str) -> Result<Session, SessionError>;
    async fn delete_session(&self, id: &str) -> Result<(), SessionError>;
    async fn list_sessions(&self) -> Result<Vec<Session>, SessionError>;
}

pub struct SessionManagerImpl;

impl SessionManager for SessionManagerImpl {
    fn new() -> Result<Self, SessionError> {
        Ok(SessionManagerImpl)
    }

    fn name(&self) -> &str {
        "session-manager"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn create_session(&self, id: String) -> Result<Session, SessionError> {
        Ok(Session {
            id,
            created_at: std::time::SystemTime::now(),
            metadata: serde_json::json!({}),
        })
    }

    async fn get_session(&self, id: &str) -> Result<Session, SessionError> {
        Err(SessionError::NotFound(id.to_string()))
    }

    async fn delete_session(&self, _id: &str) -> Result<(), SessionError> {
        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<Session>, SessionError> {
        Ok(vec![])
    }
}
