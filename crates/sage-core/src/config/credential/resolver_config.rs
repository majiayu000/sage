//! Resolver configuration
//!
//! This module defines configuration for the credential resolver.

use super::providers::{ProviderEnvConfig, default_providers};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for the credential resolver
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Working directory (for project-level config)
    pub working_dir: PathBuf,
    /// Global config directory (typically ~/.sage)
    pub global_dir: PathBuf,
    /// Provider configurations
    pub providers: Vec<ProviderEnvConfig>,
    /// CLI-provided API keys (highest priority)
    pub cli_keys: HashMap<String, String>,
    /// Whether to attempt auto-import
    pub enable_auto_import: bool,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            global_dir: dirs::home_dir()
                .unwrap_or_default()
                .join(".sage"),
            providers: default_providers(),
            cli_keys: HashMap::new(),
            enable_auto_import: true,
        }
    }
}

impl ResolverConfig {
    /// Create a new resolver config with working directory
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            ..Default::default()
        }
    }

    /// Set the global directory
    pub fn with_global_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.global_dir = dir.into();
        self
    }

    /// Add a CLI-provided API key
    pub fn with_cli_key(mut self, provider: impl Into<String>, key: impl Into<String>) -> Self {
        self.cli_keys.insert(provider.into(), key.into());
        self
    }

    /// Set whether to enable auto-import
    pub fn with_auto_import(mut self, enabled: bool) -> Self {
        self.enable_auto_import = enabled;
        self
    }

    /// Get the project credentials file path
    pub fn project_credentials_path(&self) -> PathBuf {
        self.working_dir.join(".sage").join("credentials.json")
    }

    /// Get the global credentials file path
    pub fn global_credentials_path(&self) -> PathBuf {
        self.global_dir.join("credentials.json")
    }
}
