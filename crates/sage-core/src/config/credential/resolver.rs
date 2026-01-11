//! Credential resolver for multi-source credential loading
//!
//! This module provides the CredentialResolver which loads credentials from
//! multiple sources in priority order, returning the highest-priority credential found.

use super::credentials_file::CredentialsFile;
use super::providers::auto_import_paths;
use super::resolved::{ResolvedCredential, ResolvedCredentials};
use super::resolver_config::ResolverConfig;
use super::source::CredentialSource;
use super::status::ConfigStatusReport;
use std::env;
use std::path::PathBuf;
use tracing::{debug, info};

/// Credential resolver that loads credentials from multiple sources
pub struct CredentialResolver {
    config: ResolverConfig,
}

impl CredentialResolver {
    /// Create a new credential resolver
    pub fn new(config: ResolverConfig) -> Self {
        Self { config }
    }

    /// Create a resolver with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ResolverConfig::default())
    }

    /// Create a resolver for a specific working directory
    pub fn for_directory(working_dir: impl Into<PathBuf>) -> Self {
        Self::new(ResolverConfig::new(working_dir))
    }

    /// Resolve credentials for all configured providers
    pub fn resolve_all(&self) -> ResolvedCredentials {
        let mut credentials = ResolvedCredentials::new();

        for provider_config in &self.config.providers {
            let credential = self.resolve_provider(&provider_config.name, &provider_config.env_var);
            credentials.add(credential);
        }

        credentials
    }

    /// Resolve credential for a specific provider
    pub fn resolve_provider(&self, provider: &str, env_var: &str) -> ResolvedCredential {
        // 1. Check CLI arguments (highest priority)
        if let Some(key) = self.config.cli_keys.get(provider) {
            debug!("Found {} key from CLI argument", provider);
            return ResolvedCredential::new(
                key.clone(),
                provider,
                CredentialSource::cli(format!("--{}-api-key", provider)),
            );
        }

        // 2. Check environment variables
        if let Ok(key) = env::var(env_var) {
            if !key.is_empty() {
                debug!("Found {} key from environment variable {}", provider, env_var);
                return ResolvedCredential::new(key, provider, CredentialSource::env(env_var));
            }
        }

        // 3. Check project-level credentials
        let project_path = self.config.project_credentials_path();
        if let Some(creds) = CredentialsFile::load(&project_path) {
            if let Some(key) = creds.get_api_key(provider) {
                debug!("Found {} key from project credentials", provider);
                return ResolvedCredential::new(
                    key,
                    provider,
                    CredentialSource::project(&project_path),
                );
            }
        }

        // 4. Check global credentials
        let global_path = self.config.global_credentials_path();
        if let Some(creds) = CredentialsFile::load(&global_path) {
            if let Some(key) = creds.get_api_key(provider) {
                debug!("Found {} key from global credentials", provider);
                return ResolvedCredential::new(
                    key,
                    provider,
                    CredentialSource::global(&global_path),
                );
            }
        }

        // 5. Try auto-import from other tools
        if self.config.enable_auto_import {
            for (tool_name, path) in auto_import_paths() {
                if let Some(creds) = CredentialsFile::load(&path) {
                    if let Some(key) = creds.get_api_key(provider) {
                        info!(
                            "Auto-imported {} key from {} at {}",
                            provider,
                            tool_name,
                            path.display()
                        );
                        return ResolvedCredential::new(
                            key,
                            provider,
                            CredentialSource::auto_imported(&tool_name, Some(path)),
                        );
                    }
                }
            }
        }

        // No credential found
        debug!("No credential found for {}", provider);
        ResolvedCredential::missing(provider)
    }

    /// Get the configuration status
    pub fn get_status(&self) -> ConfigStatusReport {
        let credentials = self.resolve_all();

        let configured: Vec<String> = credentials
            .configured_providers()
            .into_iter()
            .map(String::from)
            .collect();

        let missing: Vec<String> = credentials
            .missing_providers()
            .into_iter()
            .map(String::from)
            .collect();

        if missing.is_empty() && !configured.is_empty() {
            ConfigStatusReport::complete(configured)
        } else if configured.is_empty() {
            ConfigStatusReport::unconfigured()
        } else {
            ConfigStatusReport::partial(configured, missing)
        }
    }

    /// Check if the default provider is configured
    pub fn has_default_provider(&self, default_provider: &str) -> bool {
        let default_env_var = format!("{}_API_KEY", default_provider.to_uppercase());
        let env_var = self
            .config
            .providers
            .iter()
            .find(|p| p.name == default_provider)
            .map(|p| p.env_var.as_str())
            .unwrap_or(&default_env_var);

        let credential = self.resolve_provider(default_provider, env_var);
        credential.has_value()
    }

    /// Save a credential to the global credentials file
    pub fn save_credential(&self, provider: &str, key: &str) -> std::io::Result<()> {
        let path = self.config.global_credentials_path();
        let mut creds = CredentialsFile::load(&path).unwrap_or_default();
        creds.set_api_key(provider, key);
        creds.save(&path)
    }

    /// Get the resolver configuration
    pub fn config(&self) -> &ResolverConfig {
        &self.config
    }
}

impl Default for CredentialResolver {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests;
