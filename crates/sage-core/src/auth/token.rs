//! Token management and storage
//!
//! Provides secure token storage and automatic refresh

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Access token
    pub access_token: String,
    /// Refresh token (optional)
    pub refresh_token: Option<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiration time in seconds
    pub expires_in: Option<u64>,
    /// Granted scopes
    pub scope: Option<String>,
    /// When the token was issued
    pub issued_at: DateTime<Utc>,
}

impl TokenInfo {
    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_in) = self.expires_in {
            let expiry = self.issued_at + chrono::Duration::seconds(expires_in as i64);
            // Consider expired 60 seconds before actual expiry
            expiry - chrono::Duration::seconds(60) < Utc::now()
        } else {
            false
        }
    }

    /// Get remaining lifetime in seconds
    pub fn remaining_lifetime(&self) -> Option<i64> {
        self.expires_in.map(|expires_in| {
            let expiry = self.issued_at + chrono::Duration::seconds(expires_in as i64);
            (expiry - Utc::now()).num_seconds()
        })
    }
}

/// Token storage interface
pub trait TokenStorage: Send + Sync {
    /// Store token
    fn store(&self, key: &str, token: &TokenInfo) -> Result<(), TokenStorageError>;

    /// Retrieve token
    fn retrieve(&self, key: &str) -> Result<Option<TokenInfo>, TokenStorageError>;

    /// Delete token
    fn delete(&self, key: &str) -> Result<(), TokenStorageError>;

    /// List all stored token keys
    fn list_keys(&self) -> Result<Vec<String>, TokenStorageError>;
}

/// Token storage errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum TokenStorageError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Token not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

/// File-based token storage
pub struct FileTokenStorage {
    base_path: std::path::PathBuf,
}

impl FileTokenStorage {
    /// Create new file-based storage
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Get default storage location
    pub fn default_location() -> Result<Self, TokenStorageError> {
        let home = dirs::home_dir()
            .ok_or_else(|| TokenStorageError::StorageError("Cannot find home directory".into()))?;
        let path = home.join(".sage/tokens");

        // Create directory with secure permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(true).mode(0o700);
            builder.create(&path)
                .map_err(|e| TokenStorageError::IoError(e.to_string()))?;
        }

        // On non-Unix systems, use default permissions
        #[cfg(not(unix))]
        {
            std::fs::create_dir_all(&path)
                .map_err(|e| TokenStorageError::IoError(e.to_string()))?;
        }

        // Verify permissions on existing directory (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path)
                .map_err(|e| TokenStorageError::IoError(e.to_string()))?;
            let perms = metadata.permissions();
            let mode = perms.mode() & 0o777;

            // Warn if permissions are too permissive
            if mode != 0o700 {
                tracing::warn!(
                    "Token directory has insecure permissions: {:o}. Expected 0700. Attempting to fix...",
                    mode
                );
                let new_perms = std::fs::Permissions::from_mode(0o700);
                std::fs::set_permissions(&path, new_perms)
                    .map_err(|e| TokenStorageError::IoError(format!(
                        "Failed to set secure permissions on token directory: {}",
                        e
                    )))?;
            }
        }

        Ok(Self::new(path))
    }

    fn token_path(&self, key: &str) -> std::path::PathBuf {
        // Sanitize key to prevent path traversal
        let safe_key = key.replace(['/', '\\'], "_").replace("..", "_");
        self.base_path.join(format!("{}.json", safe_key))
    }
}

impl TokenStorage for FileTokenStorage {
    fn store(&self, key: &str, token: &TokenInfo) -> Result<(), TokenStorageError> {
        std::fs::create_dir_all(&self.base_path)
            .map_err(|e| TokenStorageError::IoError(e.to_string()))?;

        let path = self.token_path(key);
        let content = serde_json::to_string_pretty(token)
            .map_err(|e| TokenStorageError::SerializationError(e.to_string()))?;

        std::fs::write(&path, content)
            .map_err(|e| TokenStorageError::IoError(e.to_string()))?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms)
                .map_err(|e| TokenStorageError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<TokenInfo>, TokenStorageError> {
        let path = self.token_path(key);

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| TokenStorageError::IoError(e.to_string()))?;

        let token: TokenInfo = serde_json::from_str(&content)
            .map_err(|e| TokenStorageError::SerializationError(e.to_string()))?;

        Ok(Some(token))
    }

    fn delete(&self, key: &str) -> Result<(), TokenStorageError> {
        let path = self.token_path(key);

        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| TokenStorageError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    fn list_keys(&self) -> Result<Vec<String>, TokenStorageError> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut keys = Vec::new();
        for entry in std::fs::read_dir(&self.base_path)
            .map_err(|e| TokenStorageError::IoError(e.to_string()))?
        {
            let entry = entry.map_err(|e| TokenStorageError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    keys.push(stem.to_string_lossy().to_string());
                }
            }
        }

        Ok(keys)
    }
}

/// Token manager for handling token lifecycle
pub struct TokenManager {
    storage: Box<dyn TokenStorage>,
}

impl TokenManager {
    /// Create new token manager with storage
    pub fn new(storage: Box<dyn TokenStorage>) -> Self {
        Self { storage }
    }

    /// Create with default file storage
    pub fn with_default_storage() -> Result<Self, TokenStorageError> {
        let storage = FileTokenStorage::default_location()?;
        Ok(Self::new(Box::new(storage)))
    }

    /// Store a token
    pub fn store(&self, key: &str, token: &TokenInfo) -> Result<(), TokenStorageError> {
        self.storage.store(key, token)
    }

    /// Get a token (returns None if not found or expired)
    pub fn get(&self, key: &str) -> Result<Option<TokenInfo>, TokenStorageError> {
        match self.storage.retrieve(key)? {
            Some(token) if !token.is_expired() => Ok(Some(token)),
            Some(_) => Ok(None), // Token expired
            None => Ok(None),
        }
    }

    /// Get a token even if expired (for refresh)
    pub fn get_for_refresh(&self, key: &str) -> Result<Option<TokenInfo>, TokenStorageError> {
        self.storage.retrieve(key)
    }

    /// Delete a token
    pub fn delete(&self, key: &str) -> Result<(), TokenStorageError> {
        self.storage.delete(key)
    }

    /// List all stored tokens
    pub fn list(&self) -> Result<Vec<String>, TokenStorageError> {
        self.storage.list_keys()
    }

    /// Clear all tokens
    pub fn clear_all(&self) -> Result<(), TokenStorageError> {
        for key in self.list()? {
            self.delete(&key)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_token_expiry() {
        let token = TokenInfo {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            scope: None,
            issued_at: Utc::now(),
        };

        assert!(!token.is_expired());
        assert!(token.remaining_lifetime().unwrap() > 3500);
    }

    #[test]
    fn test_token_expired() {
        let token = TokenInfo {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: Some(30), // 30 seconds
            scope: None,
            issued_at: Utc::now() - chrono::Duration::seconds(60), // Issued 60 seconds ago
        };

        assert!(token.is_expired());
    }

    #[test]
    fn test_file_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileTokenStorage::new(temp_dir.path());

        let token = TokenInfo {
            access_token: "test_token".to_string(),
            refresh_token: Some("refresh".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            scope: Some("read write".to_string()),
            issued_at: Utc::now(),
        };

        // Store
        storage.store("test_key", &token).unwrap();

        // Retrieve
        let retrieved = storage.retrieve("test_key").unwrap().unwrap();
        assert_eq!(retrieved.access_token, "test_token");

        // List
        let keys = storage.list_keys().unwrap();
        assert_eq!(keys, vec!["test_key"]);

        // Delete
        storage.delete("test_key").unwrap();
        assert!(storage.retrieve("test_key").unwrap().is_none());
    }
}
