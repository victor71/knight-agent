//! Secret Manager
//!
//! Handles secure storage and retrieval of secrets/keys.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::types::*;

/// Secret entry with encrypted value
struct SecretEntry {
    encrypted_value: Vec<u8>,
    info: SecretInfo,
}

/// Secret manager for secure key storage
pub struct SecretManager {
    secrets: Arc<RwLock<HashMap<String, SecretEntry>>>,
    policy: SecretPolicyConfig,
}

impl SecretManager {
    pub fn new(policy: SecretPolicyConfig) -> Self {
        Self {
            secrets: Arc::new(RwLock::new(HashMap::new())),
            policy,
        }
    }

    /// Store a secret
    pub async fn store_secret(
        &self,
        key: String,
        value: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<bool, SecretError> {
        // Validate secret
        self.validate_secret(&value)?;

        let now = std::time::SystemTime::now();

        // In a real implementation, value would be encrypted here
        // For now, we store it (in production, use proper encryption)
        let entry = SecretEntry {
            encrypted_value: value.into_bytes(),
            info: SecretInfo {
                key: key.clone(),
                created_at: now,
                updated_at: now,
                metadata: metadata.unwrap_or_default(),
            },
        };

        let mut secrets = self.secrets.write().await;
        secrets.insert(key, entry);

        Ok(true)
    }

    /// Retrieve a secret value
    pub async fn get_secret(&self, key: &str) -> Result<Option<String>, SecretError> {
        let secrets = self.secrets.read().await;

        match secrets.get(key) {
            Some(entry) => {
                // In a real implementation, value would be decrypted here
                let value = String::from_utf8(entry.encrypted_value.clone())
                    .map_err(|_| SecretError::DecryptionFailed)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Delete a secret
    pub async fn delete_secret(&self, key: &str) -> Result<bool, SecretError> {
        let mut secrets = self.secrets.write().await;
        Ok(secrets.remove(key).is_some())
    }

    /// List secret metadata (without values)
    pub async fn list_secrets(&self) -> Vec<SecretInfo> {
        let secrets = self.secrets.read().await;
        secrets.values().map(|e| e.info.clone()).collect()
    }

    /// Rotate a secret
    pub async fn rotate_secret(
        &self,
        key: &str,
        new_value: String,
    ) -> Result<bool, SecretError> {
        // Validate new secret
        self.validate_secret(&new_value)?;

        let mut secrets = self.secrets.write().await;

        match secrets.get_mut(key) {
            Some(entry) => {
                // In a real implementation, new_value would be encrypted here
                entry.encrypted_value = new_value.into_bytes();
                entry.info.updated_at = std::time::SystemTime::now();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Validate secret against policy
    fn validate_secret(&self, value: &str) -> Result<(), SecretError> {
        // Check minimum length
        if value.len() < self.policy.min_length {
            return Err(SecretError::ValidationFailed(format!(
                "secret too short: minimum length is {}",
                self.policy.min_length
            )));
        }

        // Check for special characters if required
        if self.policy.require_special_chars {
            let has_special = value.chars().any(|c| !c.is_alphanumeric());
            if !has_special {
                return Err(SecretError::ValidationFailed(
                    "secret must contain special characters".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Secret manager errors
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("secret not found: {0}")]
    NotFound(String),

    #[error("validation failed: {0}")]
    ValidationFailed(String),

    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed")]
    DecryptionFailed,

    #[error("access denied")]
    AccessDenied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve_secret() {
        let manager = SecretManager::new(SecretPolicyConfig::default());

        let result = manager
            .store_secret("api-key".to_string(), "supersecretkey123!".to_string(), None)
            .await;
        assert!(result.is_ok());

        let value = manager.get_secret("api-key").await.unwrap();
        assert_eq!(value, Some("supersecretkey123!".to_string()));
    }

    #[tokio::test]
    async fn test_secret_not_found() {
        let manager = SecretManager::new(SecretPolicyConfig::default());

        let value = manager.get_secret("nonexistent").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_delete_secret() {
        let manager = SecretManager::new(SecretPolicyConfig::default());

        manager
            .store_secret("temp-key".to_string(), "temp_value_1234!".to_string(), None)
            .await
            .unwrap();

        let result = manager.delete_secret("temp-key").await.unwrap();
        assert!(result);

        let value = manager.get_secret("temp-key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_validate_secret_length() {
        let policy = SecretPolicyConfig {
            min_length: 16,
            ..Default::default()
        };
        let manager = SecretManager::new(policy);

        let result = manager
            .store_secret("short-key".to_string(), "tooshort".to_string(), None)
            .await;

        assert!(result.is_err());
    }
}
