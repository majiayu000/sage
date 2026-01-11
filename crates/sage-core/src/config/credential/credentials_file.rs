//! Credentials file management
//!
//! This module handles loading and saving credentials from JSON files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

/// Credentials stored in a JSON file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CredentialsFile {
    /// API keys indexed by provider name
    #[serde(default)]
    pub api_keys: HashMap<String, String>,

    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl CredentialsFile {
    /// Load credentials from a file
    pub fn load(path: &Path) -> Option<Self> {
        if !path.exists() {
            return None;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(creds) => Some(creds),
                Err(e) => {
                    warn!("Failed to parse credentials file {}: {}", path.display(), e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read credentials file {}: {}", path.display(), e);
                None
            }
        }
    }

    /// Save credentials to a file
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    /// Get an API key for a provider
    pub fn get_api_key(&self, provider: &str) -> Option<&str> {
        self.api_keys.get(provider).map(|s| s.as_str())
    }

    /// Set an API key for a provider
    pub fn set_api_key(&mut self, provider: impl Into<String>, key: impl Into<String>) {
        self.api_keys.insert(provider.into(), key.into());
    }
}
