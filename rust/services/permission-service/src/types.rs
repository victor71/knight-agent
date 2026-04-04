//! Permission types

use serde::{Deserialize, Serialize};

/// Permission structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub user_id: String,
    pub resource: String,
    pub action: String,
}

impl Permission {
    /// Create a new permission
    pub fn new(user_id: String, resource: String, action: String) -> Self {
        Self {
            user_id,
            resource,
            action,
        }
    }
}
