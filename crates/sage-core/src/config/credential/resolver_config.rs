//! Resolver configuration
//!
//! This module defines configuration for the credential resolver.

use super::backend::{CredentialBackend, UnsupportedCredentialBackend};
use super::providers::{ProviderEnvConfig, default_providers};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

/// Configuration for the credential resolver
#[derive(Clone)]
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
    /// Whether legacy plaintext JSON credentials are accepted as fallback.
    pub allow_legacy_plaintext: bool,
    /// Durable secure credential backend.
    pub credential_backend: Arc<dyn CredentialBackend>,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            global_dir: crate::config::default_data_dir_or_warn(),
            providers: default_providers(),
            cli_keys: HashMap::new(),
            enable_auto_import: true,
            allow_legacy_plaintext: true,
            credential_backend: Arc::new(UnsupportedCredentialBackend),
        }
    }
}

impl fmt::Debug for ResolverConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolverConfig")
            .field("working_dir", &self.working_dir)
            .field("global_dir", &self.global_dir)
            .field("providers", &self.providers)
            .field("cli_keys", &self.cli_keys.keys().collect::<Vec<_>>())
            .field("enable_auto_import", &self.enable_auto_import)
            .field("allow_legacy_plaintext", &self.allow_legacy_plaintext)
            .field("credential_backend", &self.credential_backend.kind())
            .finish()
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

    /// Set whether legacy plaintext JSON credentials are accepted as fallback.
    pub fn with_legacy_plaintext(mut self, enabled: bool) -> Self {
        self.allow_legacy_plaintext = enabled;
        self
    }

    /// Set the secure credential backend.
    pub fn with_credential_backend(mut self, backend: Arc<dyn CredentialBackend>) -> Self {
        self.credential_backend = backend;
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
