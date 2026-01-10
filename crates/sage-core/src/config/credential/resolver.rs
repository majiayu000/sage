//! Credential resolver for multi-source credential loading
//!
//! This module provides the CredentialResolver which loads credentials from
//! multiple sources in priority order, returning the highest-priority credential found.

use super::resolved::{ResolvedCredential, ResolvedCredentials};
use super::source::CredentialSource;
use super::status::ConfigStatusReport;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Known provider configurations for environment variable names
#[derive(Debug, Clone)]
pub struct ProviderEnvConfig {
    /// The provider name
    pub name: String,
    /// Environment variable name for API key
    pub env_var: String,
}

impl ProviderEnvConfig {
    pub fn new(name: impl Into<String>, env_var: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            env_var: env_var.into(),
        }
    }
}

/// Default provider configurations
pub fn default_providers() -> Vec<ProviderEnvConfig> {
    vec![
        ProviderEnvConfig::new("anthropic", "ANTHROPIC_API_KEY"),
        ProviderEnvConfig::new("openai", "OPENAI_API_KEY"),
        ProviderEnvConfig::new("google", "GOOGLE_API_KEY"),
        ProviderEnvConfig::new("glm", "GLM_API_KEY"),
        ProviderEnvConfig::new("zhipu", "ZHIPU_API_KEY"),
        ProviderEnvConfig::new("ollama", "OLLAMA_API_KEY"),
    ]
}

/// Paths to check for auto-import from other tools
pub fn auto_import_paths() -> Vec<(String, PathBuf)> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        // Claude Code
        (
            "claude-code".to_string(),
            home.join(".claude").join("credentials.json"),
        ),
        // Cursor
        (
            "cursor".to_string(),
            home.join(".cursor").join("credentials.json"),
        ),
        // Aider
        (
            "aider".to_string(),
            home.join(".aider").join("credentials.json"),
        ),
    ]
}

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
mod tests {
    use super::*;
    use super::super::status::ConfigStatus;
    use serial_test::serial;
    use tempfile::tempdir;

    fn clean_env() {
        unsafe {
            env::remove_var("ANTHROPIC_API_KEY");
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("GOOGLE_API_KEY");
            env::remove_var("GLM_API_KEY");
            env::remove_var("OLLAMA_API_KEY");
        }
    }

    #[test]
    fn test_provider_env_config() {
        let config = ProviderEnvConfig::new("test", "TEST_KEY");
        assert_eq!(config.name, "test");
        assert_eq!(config.env_var, "TEST_KEY");
    }

    #[test]
    fn test_default_providers() {
        let providers = default_providers();
        assert_eq!(providers.len(), 6);

        let names: Vec<&str> = providers.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"anthropic"));
        assert!(names.contains(&"openai"));
        assert!(names.contains(&"google"));
        assert!(names.contains(&"glm"));
        assert!(names.contains(&"zhipu"));
        assert!(names.contains(&"ollama"));
    }

    #[test]
    fn test_credentials_file_empty() {
        let creds = CredentialsFile::default();
        assert!(creds.api_keys.is_empty());
        assert!(creds.metadata.is_empty());
    }

    #[test]
    fn test_credentials_file_set_get() {
        let mut creds = CredentialsFile::default();
        creds.set_api_key("openai", "test-key");

        assert_eq!(creds.get_api_key("openai"), Some("test-key"));
        assert_eq!(creds.get_api_key("anthropic"), None);
    }

    #[test]
    fn test_credentials_file_load_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("credentials.json");

        let mut creds = CredentialsFile::default();
        creds.set_api_key("openai", "test-key-123");
        creds.save(&path).unwrap();

        let loaded = CredentialsFile::load(&path).unwrap();
        assert_eq!(loaded.get_api_key("openai"), Some("test-key-123"));
    }

    #[test]
    fn test_credentials_file_load_nonexistent() {
        let loaded = CredentialsFile::load(Path::new("/nonexistent/path.json"));
        assert!(loaded.is_none());
    }

    #[test]
    fn test_resolver_config_default() {
        let config = ResolverConfig::default();
        assert!(!config.providers.is_empty());
        assert!(config.cli_keys.is_empty());
        assert!(config.enable_auto_import);
    }

    #[test]
    fn test_resolver_config_builder() {
        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir("/global")
            .with_cli_key("openai", "cli-key")
            .with_auto_import(false);

        assert_eq!(config.working_dir, dir.path());
        assert_eq!(config.global_dir, PathBuf::from("/global"));
        assert_eq!(config.cli_keys.get("openai"), Some(&"cli-key".to_string()));
        assert!(!config.enable_auto_import);
    }

    #[test]
    fn test_resolver_config_paths() {
        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path()).with_global_dir(dir.path().join("global"));

        assert!(config
            .project_credentials_path()
            .ends_with(".sage/credentials.json"));
        assert!(config
            .global_credentials_path()
            .ends_with("global/credentials.json"));
    }

    #[test]
    #[serial]
    fn test_resolver_cli_priority() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_cli_key("openai", "cli-key");

        // Also set env var - should be ignored
        unsafe {
            env::set_var("OPENAI_API_KEY", "env-key");
        }

        let resolver = CredentialResolver::new(config);
        let cred = resolver.resolve_provider("openai", "OPENAI_API_KEY");

        assert_eq!(cred.value(), Some("cli-key"));
        assert!(matches!(
            cred.source,
            CredentialSource::CliArgument { .. }
        ));

        clean_env();
    }

    #[test]
    #[serial]
    fn test_resolver_env_priority() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"));

        unsafe {
            env::set_var("OPENAI_API_KEY", "env-key");
        }

        let resolver = CredentialResolver::new(config);
        let cred = resolver.resolve_provider("openai", "OPENAI_API_KEY");

        assert_eq!(cred.value(), Some("env-key"));
        assert!(matches!(
            cred.source,
            CredentialSource::Environment { .. }
        ));

        clean_env();
    }

    #[test]
    #[serial]
    fn test_resolver_project_config_priority() {
        clean_env();

        let dir = tempdir().unwrap();

        // Create project credentials
        let project_creds_dir = dir.path().join(".sage");
        std::fs::create_dir_all(&project_creds_dir).unwrap();
        let mut creds = CredentialsFile::default();
        creds.set_api_key("openai", "project-key");
        creds
            .save(&project_creds_dir.join("credentials.json"))
            .unwrap();

        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"));

        let resolver = CredentialResolver::new(config);
        let cred = resolver.resolve_provider("openai", "OPENAI_API_KEY");

        assert_eq!(cred.value(), Some("project-key"));
        assert!(matches!(
            cred.source,
            CredentialSource::ProjectConfig { .. }
        ));
    }

    #[test]
    #[serial]
    fn test_resolver_global_config_priority() {
        clean_env();

        let dir = tempdir().unwrap();

        // Create global credentials
        let global_dir = dir.path().join("global");
        std::fs::create_dir_all(&global_dir).unwrap();
        let mut creds = CredentialsFile::default();
        creds.set_api_key("openai", "global-key");
        creds.save(&global_dir.join("credentials.json")).unwrap();

        let config = ResolverConfig::new(dir.path()).with_global_dir(&global_dir);

        let resolver = CredentialResolver::new(config);
        let cred = resolver.resolve_provider("openai", "OPENAI_API_KEY");

        assert_eq!(cred.value(), Some("global-key"));
        assert!(matches!(
            cred.source,
            CredentialSource::GlobalConfig { .. }
        ));
    }

    #[test]
    #[serial]
    fn test_resolver_missing_credential() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_auto_import(false);

        let resolver = CredentialResolver::new(config);
        let cred = resolver.resolve_provider("openai", "OPENAI_API_KEY");

        assert!(cred.is_missing());
    }

    #[test]
    #[serial]
    fn test_resolver_resolve_all() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_cli_key("openai", "openai-key")
            .with_auto_import(false);

        unsafe {
            env::set_var("ANTHROPIC_API_KEY", "anthropic-key");
        }

        let resolver = CredentialResolver::new(config);
        let all = resolver.resolve_all();

        assert_eq!(all.get_api_key("openai"), Some("openai-key"));
        assert_eq!(all.get_api_key("anthropic"), Some("anthropic-key"));
        assert!(all.get_api_key("google").is_none());

        clean_env();
    }

    #[test]
    #[serial]
    fn test_resolver_get_status_complete() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_cli_key("anthropic", "key1")
            .with_cli_key("openai", "key2")
            .with_cli_key("google", "key3")
            .with_cli_key("glm", "key4")
            .with_cli_key("zhipu", "key5")
            .with_cli_key("ollama", "key6")
            .with_auto_import(false);

        let resolver = CredentialResolver::new(config);
        let status = resolver.get_status();

        assert_eq!(status.status, ConfigStatus::Complete);
        assert_eq!(status.configured_providers.len(), 6);
        assert!(status.missing_credentials.is_empty());
    }

    #[test]
    #[serial]
    fn test_resolver_get_status_partial() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_cli_key("openai", "key")
            .with_auto_import(false);

        let resolver = CredentialResolver::new(config);
        let status = resolver.get_status();

        assert_eq!(status.status, ConfigStatus::Partial);
        assert!(status.configured_providers.contains(&"openai".to_string()));
        assert!(!status.missing_credentials.is_empty());
    }

    #[test]
    #[serial]
    fn test_resolver_get_status_unconfigured() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_auto_import(false);

        let resolver = CredentialResolver::new(config);
        let status = resolver.get_status();

        assert_eq!(status.status, ConfigStatus::Unconfigured);
    }

    #[test]
    #[serial]
    fn test_resolver_has_default_provider() {
        clean_env();

        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_cli_key("openai", "key")
            .with_auto_import(false);

        let resolver = CredentialResolver::new(config);

        assert!(resolver.has_default_provider("openai"));
        assert!(!resolver.has_default_provider("anthropic"));
    }

    #[test]
    fn test_resolver_save_credential() {
        let dir = tempdir().unwrap();
        let config = ResolverConfig::new(dir.path())
            .with_global_dir(dir.path().join("global"));

        let resolver = CredentialResolver::new(config);
        resolver.save_credential("openai", "saved-key").unwrap();

        // Verify it was saved
        let creds = CredentialsFile::load(&dir.path().join("global/credentials.json")).unwrap();
        assert_eq!(creds.get_api_key("openai"), Some("saved-key"));
    }

    #[test]
    fn test_resolver_default() {
        let resolver = CredentialResolver::default();
        assert!(!resolver.config().providers.is_empty());
    }

    #[test]
    fn test_resolver_for_directory() {
        let dir = tempdir().unwrap();
        let resolver = CredentialResolver::for_directory(dir.path());
        assert_eq!(resolver.config().working_dir, dir.path());
    }
}
