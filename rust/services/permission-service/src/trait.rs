//! Permission Service trait

use async_trait::async_trait;

use crate::error::PermissionResult;
use crate::types::Permission;

/// Permission Service trait
#[async_trait::async_trait]
pub trait PermissionService: Send + Sync {
    /// Create a new permission service
    fn new() -> PermissionResult<Self>
    where
        Self: Sized;

    /// Get the name of this service
    fn name(&self) -> &str;

    /// Check if the service is initialized
    fn is_initialized(&self) -> bool;

    /// Initialize the service
    async fn initialize(&self) -> PermissionResult<()>;

    /// Check if a permission is granted
    async fn check_permission(&self, permission: Permission) -> PermissionResult<bool>;

    /// Grant a permission
    async fn grant_permission(&self, permission: Permission) -> PermissionResult<()>;

    /// Revoke a permission
    async fn revoke_permission(&self, permission: Permission) -> PermissionResult<()>;
}
