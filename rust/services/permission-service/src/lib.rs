//! Permission Service
//!
//! Design Reference: docs/03-module-design/services/permission-service.md

#![allow(unused)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PermissionError {
    #[error("Permission not initialized")]
    NotInitialized,
    #[error("Permission denied: {0}")]
    Denied(String),
    #[error("Permission check failed: {0}")]
    CheckFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub user_id: String,
    pub resource: String,
    pub action: String,
}

pub trait PermissionService: Send + Sync {
    fn new() -> Result<Self, PermissionError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn check_permission(&self, permission: Permission) -> Result<bool, PermissionError>;
    async fn grant_permission(&self, permission: Permission) -> Result<(), PermissionError>;
    async fn revoke_permission(&self, permission: Permission) -> Result<(), PermissionError>;
}

pub struct PermissionServiceImpl;

impl PermissionService for PermissionServiceImpl {
    fn new() -> Result<Self, PermissionError> {
        Ok(PermissionServiceImpl)
    }

    fn name(&self) -> &str {
        "permission-service"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn check_permission(&self, _permission: Permission) -> Result<bool, PermissionError> {
        Ok(false)
    }

    async fn grant_permission(&self, _permission: Permission) -> Result<(), PermissionError> {
        Ok(())
    }

    async fn revoke_permission(&self, _permission: Permission) -> Result<(), PermissionError> {
        Ok(())
    }
}
