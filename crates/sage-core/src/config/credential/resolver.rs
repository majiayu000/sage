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
                debug!(
                    "Found {} key from environment variable {}",
                    provider, env_var
                );
                return ResolvedCredential::new(key, provider, CredentialSource::env(env_var));
            }
        }

        // 3. Check project-level credentials
        let project_path = self.config.project_credentials_path();
        if let Some(creds) = CredentialsFile::load_or_warn(&project_path) {
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
        if let Some(creds) = CredentialsFile::load_or_warn(&global_path) {
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
                if let Some(creds) = CredentialsFile::load_or_warn(&path) {
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

    /// Check if the default provider is configured.
    ///
    /// Tries every standard env-var name for the provider (e.g. Azure
    /// accepts both `AZURE_OPENAI_API_KEY` and `AZURE_API_KEY`), plus
    /// the resolver's own configured `env_var`, plus the generic
    /// `{PROVIDER}_API_KEY` fallback. Returns `true` as soon as any of
    /// them resolves to a value.
    ///
    /// This avoids a regression introduced when explicit Azure /
    /// OpenRouter entries were added to the resolver: previously
    /// `has_default_provider("azure")` fell back to the generated
    /// `AZURE_API_KEY`; without the multi-env-var probe, adding an
    /// `AZURE_OPENAI_API_KEY` entry would cause a user with only the
    /// long-supported `AZURE_API_KEY` set to be reported as
    /// unconfigured.
    pub fn has_default_provider(&self, default_provider: &str) -> bool {
        use crate::config::api_key_helpers::get_standard_env_vars_for_provider;

        // 1. The resolver's own configured env_var, if any.
        let configured_env_var = self
            .config
            .providers
            .iter()
            .find(|p| p.name == default_provider)
            .map(|p| p.env_var.clone());

        // 2. The published standard list for the provider (covers
        //    aliases like AZURE_API_KEY, CLAUDE_API_KEY, GEMINI_API_KEY).
        let mut candidates: Vec<String> = get_standard_env_vars_for_provider(default_provider);

        // Merge the configured one to the front so it takes priority.
        if let Some(configured) = configured_env_var {
            if !candidates.iter().any(|c| c == &configured) {
                candidates.insert(0, configured);
            }
        }

        // 3. Generic fallback so the loop below is never empty.
        let generic = format!("{}_API_KEY", default_provider.to_uppercase());
        if !candidates.iter().any(|c| c == &generic) {
            candidates.push(generic);
        }

        candidates
            .iter()
            .any(|env_var| self.resolve_provider(default_provider, env_var).has_value())
    }

    /// Save a credential to the global credentials file
    pub fn save_credential(&self, provider: &str, key: &str) -> std::io::Result<()> {
        let path = self.config.global_credentials_path();
        let mut creds = CredentialsFile::load(&path)?.unwrap_or_default();
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
