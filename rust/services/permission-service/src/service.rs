//! Permission Service
//!
//! Permission management for Knight-Agent.
//! Handles permission checking, granting, and revoking.
//!
//! Design Reference: docs/03-module-design/services/permission-service.md

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{PermissionError, PermissionResult};
use crate::r#trait::PermissionService;
use crate::types::Permission;

/// Permission service implementation
#[derive(Clone)]
pub struct PermissionServiceImpl {
    permissions: Arc<RwLock<HashSet<String>>>,
    initialized: Arc<RwLock<bool>>,
}

impl PermissionServiceImpl {
    /// Create a new permission service
    pub fn new() -> PermissionResult<Self> {
        Ok(Self {
            permissions: Arc::new(RwLock::new(HashSet::new())),
            initialized: Arc::new(RwLock::new(false)),
        })
    }

    /// Generate permission key
    fn permission_key(permission: &Permission) -> String {
        format!(
            "{}:{}:{}",
            permission.user_id, permission.resource, permission.action
        )
    }
}

impl Default for PermissionServiceImpl {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[async_trait::async_trait]
impl PermissionService for PermissionServiceImpl {
    fn new() -> PermissionResult<Self> {
        Self::new()
    }

    fn name(&self) -> &str {
        "permission-service"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
            .try_read()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    async fn initialize(&self) -> PermissionResult<()> {
        *self.initialized.write().await = true;
        tracing::info!("Permission service initialized");
        Ok(())
    }

    async fn check_permission(&self, permission: Permission) -> PermissionResult<bool> {
        if !self.is_initialized() {
            return Err(PermissionError::NotInitialized);
        }

        let key = Self::permission_key(&permission);
        Ok(self.permissions.read().await.contains(&key))
    }

    async fn grant_permission(&self, permission: Permission) -> PermissionResult<()> {
        if !self.is_initialized() {
            return Err(PermissionError::NotInitialized);
        }

        let key = Self::permission_key(&permission);
        tracing::debug!("Granting permission: {}", key);
        self.permissions.write().await.insert(key);
        Ok(())
    }

    async fn revoke_permission(&self, permission: Permission) -> PermissionResult<()> {
        if !self.is_initialized() {
            return Err(PermissionError::NotInitialized);
        }

        let key = Self::permission_key(&permission);
        tracing::debug!("Revoking permission: {}", key);
        self.permissions.write().await.remove(&key);
        Ok(())
    }
}
