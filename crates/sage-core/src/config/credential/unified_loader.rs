//! Unified configuration loader that never fails
//!
//! This module provides a configuration loader that gracefully handles missing
//! or invalid configuration, returning usable defaults with status information.

use super::resolver::{CredentialResolver, ResolverConfig};
use super::status::ConfigStatusReport;
use crate::config::model::Config;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Result of loading configuration
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    /// The loaded configuration (always valid, may be defaults)
    pub config: Config,
    /// Status of the configuration
    pub status: ConfigStatusReport,
    /// Path to the config file that was loaded (if any)
    pub config_file: Option<PathBuf>,
    /// Warnings encountered during loading
    pub warnings: Vec<String>,
}

impl LoadedConfig {
    /// Create a new loaded config
    pub fn new(config: Config, status: ConfigStatusReport) -> Self {
        Self {
            config,
            status,
            config_file: None,
            warnings: Vec::new(),
        }
    }

    /// Set the config file path
    pub fn with_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_file = Some(path.into());
        self
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Check if the configuration is ready to use
    pub fn is_ready(&self) -> bool {
        self.status.status.is_ready()
    }

    /// Check if onboarding should be triggered
    pub fn needs_onboarding(&self) -> bool {
        self.status.status.needs_onboarding()
    }

    /// Get a user-facing message about the configuration status
    pub fn status_message(&self) -> &str {
        &self.status.message
    }

    /// Get a suggestion for the user
    pub fn suggestion(&self) -> Option<&str> {
        self.status.suggestion.as_deref()
    }
}

/// Configuration loader that never fails
///
/// This loader:
/// 1. Attempts to load configuration from the specified file
/// 2. Falls back to environment variables
/// 3. Falls back to defaults
/// 4. Always returns a valid Config with status information
pub struct UnifiedConfigLoader {
    /// Path to the config file
    config_file: Option<PathBuf>,
    /// Working directory
    working_dir: PathBuf,
    /// Global config directory
    global_dir: PathBuf,
    /// CLI-provided overrides
    cli_overrides: CliOverrides,
}

/// CLI-provided configuration overrides
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    /// Provider specified via CLI
    pub provider: Option<String>,
    /// Model specified via CLI
    pub model: Option<String>,
    /// API key specified via CLI
    pub api_key: Option<String>,
    /// Max steps specified via CLI
    pub max_steps: Option<u32>,
}

impl CliOverrides {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Check if any overrides are set
    pub fn has_overrides(&self) -> bool {
        self.provider.is_some()
            || self.model.is_some()
            || self.api_key.is_some()
            || self.max_steps.is_some()
    }
}

impl UnifiedConfigLoader {
    /// Create a new unified config loader
    pub fn new() -> Self {
        Self {
            config_file: None,
            working_dir: std::env::current_dir().unwrap_or_default(),
            global_dir: dirs::home_dir()
                .unwrap_or_default()
                .join(".sage"),
            cli_overrides: CliOverrides::default(),
        }
    }

    /// Set the config file path
    pub fn with_config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_file = Some(path.into());
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = dir.into();
        self
    }

    /// Set the global config directory
    pub fn with_global_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.global_dir = dir.into();
        self
    }

    /// Set CLI overrides
    pub fn with_cli_overrides(mut self, overrides: CliOverrides) -> Self {
        self.cli_overrides = overrides;
        self
    }

    /// Load configuration - never fails, always returns valid config
    pub fn load(&self) -> LoadedConfig {
        let mut warnings = Vec::new();
        let mut config = Config::default();
        let mut config_file_used: Option<PathBuf> = None;

        // 1. Try to load from specified config file
        if let Some(ref path) = self.config_file {
            if path.exists() {
                match self.load_config_file(path) {
                    Ok(file_config) => {
                        debug!("Loaded config from {}", path.display());
                        config.merge(file_config);
                        config_file_used = Some(path.clone());
                    }
                    Err(e) => {
                        warn!("Failed to load config file {}: {}", path.display(), e);
                        warnings.push(format!(
                            "Config file {} could not be loaded: {}",
                            path.display(),
                            e
                        ));
                    }
                }
            } else {
                debug!("Config file {} not found, using defaults", path.display());
                warnings.push(format!(
                    "Config file {} not found, using defaults",
                    path.display()
                ));
            }
        }

        // 2. Try to load from default locations if no file specified or found
        if config_file_used.is_none() {
            // Try project-level config
            let project_config = self.working_dir.join("sage_config.json");
            if project_config.exists() {
                if let Ok(file_config) = self.load_config_file(&project_config) {
                    debug!("Loaded project config from {}", project_config.display());
                    config.merge(file_config);
                    config_file_used = Some(project_config);
                }
            }

            // Try global config
            if config_file_used.is_none() {
                let global_config = self.global_dir.join("config.json");
                if global_config.exists() {
                    if let Ok(file_config) = self.load_config_file(&global_config) {
                        debug!("Loaded global config from {}", global_config.display());
                        config.merge(file_config);
                        config_file_used = Some(global_config);
                    }
                }
            }
        }

        // 3. Apply CLI overrides
        self.apply_cli_overrides(&mut config);

        // 4. Resolve credentials and update config
        let resolver = self.create_credential_resolver();
        let credentials = resolver.resolve_all();

        // Update config with resolved credentials
        for cred in credentials.iter() {
            if let Some(value) = cred.value() {
                if let Some(params) = config.model_providers.get_mut(&cred.provider) {
                    if params.api_key.is_none()
                        || params.api_key.as_deref() == Some("")
                        || params.api_key.as_deref().map(|k| k.starts_with("${")).unwrap_or(false)
                    {
                        params.api_key = Some(value.to_string());
                        debug!(
                            "Applied {} API key from {}",
                            cred.provider,
                            cred.source.priority().name()
                        );
                    }
                }
            }
        }

        // 5. Get configuration status
        let status = resolver.get_status();

        // Build result
        let mut result = LoadedConfig::new(config, status);
        if let Some(path) = config_file_used {
            result = result.with_file(path);
        }
        for warning in warnings {
            result = result.with_warning(warning);
        }

        result
    }

    /// Load a config file
    fn load_config_file(&self, path: &Path) -> Result<Config, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Try JSON first
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse JSON: {}", e))
        } else {
            // Try to detect format
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {}", e))
        }
    }

    /// Apply CLI overrides to the config
    fn apply_cli_overrides(&self, config: &mut Config) {
        if let Some(ref provider) = self.cli_overrides.provider {
            config.default_provider = provider.clone();
        }

        if let Some(max_steps) = self.cli_overrides.max_steps {
            config.max_steps = Some(max_steps);
        }

        // Apply model and API key to the default provider
        let default_provider = config.default_provider.clone();
        if let Some(params) = config.model_providers.get_mut(&default_provider) {
            if let Some(ref model) = self.cli_overrides.model {
                params.model = model.clone();
            }
            if let Some(ref api_key) = self.cli_overrides.api_key {
                params.api_key = Some(api_key.clone());
            }
        }
    }

    /// Create a credential resolver based on current settings
    fn create_credential_resolver(&self) -> CredentialResolver {
        let mut config = ResolverConfig::new(&self.working_dir)
            .with_global_dir(&self.global_dir);

        // Add CLI API key if provided
        if let Some(ref api_key) = self.cli_overrides.api_key {
            let provider = self
                .cli_overrides
                .provider
                .as_deref()
                .unwrap_or("anthropic");
            config = config.with_cli_key(provider, api_key);
        }

        CredentialResolver::new(config)
    }
}

impl Default for UnifiedConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to load configuration with defaults
pub fn load_config_unified(config_file: Option<&str>) -> LoadedConfig {
    let mut loader = UnifiedConfigLoader::new();
    if let Some(path) = config_file {
        loader = loader.with_config_file(path);
    }
    loader.load()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::tempdir;

    fn clean_env() {
        unsafe {
            env::remove_var("ANTHROPIC_API_KEY");
            env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn test_loaded_config_new() {
        let config = Config::default();
        let status = ConfigStatusReport::unconfigured();
        let loaded = LoadedConfig::new(config, status);

        assert!(!loaded.is_ready());
        assert!(loaded.needs_onboarding());
    }

    #[test]
    fn test_loaded_config_with_file() {
        let config = Config::default();
        let status = ConfigStatusReport::complete(vec!["anthropic".to_string()]);
        let loaded = LoadedConfig::new(config, status).with_file("/path/to/config.json");

        assert!(loaded.config_file.is_some());
        assert_eq!(
            loaded.config_file.unwrap(),
            PathBuf::from("/path/to/config.json")
        );
    }

    #[test]
    fn test_loaded_config_with_warning() {
        let config = Config::default();
        let status = ConfigStatusReport::partial(vec![], vec!["openai".to_string()]);
        let loaded = LoadedConfig::new(config, status)
            .with_warning("Warning 1")
            .with_warning("Warning 2");

        assert_eq!(loaded.warnings.len(), 2);
    }

    #[test]
    fn test_loaded_config_status_methods() {
        let config = Config::default();

        let complete = LoadedConfig::new(
            config.clone(),
            ConfigStatusReport::complete(vec!["anthropic".to_string()]),
        );
        assert!(complete.is_ready());
        assert!(!complete.needs_onboarding());

        let partial = LoadedConfig::new(
            config.clone(),
            ConfigStatusReport::partial(vec!["anthropic".to_string()], vec!["openai".to_string()]),
        );
        assert!(partial.is_ready());
        assert!(!partial.needs_onboarding());

        let unconfigured = LoadedConfig::new(config, ConfigStatusReport::unconfigured());
        assert!(!unconfigured.is_ready());
        assert!(unconfigured.needs_onboarding());
    }

    #[test]
    fn test_cli_overrides_default() {
        let overrides = CliOverrides::default();
        assert!(!overrides.has_overrides());
    }

    #[test]
    fn test_cli_overrides_builder() {
        let overrides = CliOverrides::new()
            .with_provider("openai")
            .with_model("gpt-4")
            .with_api_key("test-key")
            .with_max_steps(50);

        assert!(overrides.has_overrides());
        assert_eq!(overrides.provider, Some("openai".to_string()));
        assert_eq!(overrides.model, Some("gpt-4".to_string()));
        assert_eq!(overrides.api_key, Some("test-key".to_string()));
        assert_eq!(overrides.max_steps, Some(50));
    }

    #[test]
    fn test_unified_loader_new() {
        let loader = UnifiedConfigLoader::new();
        assert!(loader.config_file.is_none());
    }

    #[test]
    fn test_unified_loader_builder() {
        let dir = tempdir().unwrap();
        let loader = UnifiedConfigLoader::new()
            .with_config_file("config.json")
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_cli_overrides(CliOverrides::new().with_provider("openai"));

        assert_eq!(loader.config_file, Some(PathBuf::from("config.json")));
        assert_eq!(loader.working_dir, dir.path());
        assert_eq!(loader.global_dir, dir.path().join("global"));
        assert_eq!(loader.cli_overrides.provider, Some("openai".to_string()));
    }

    #[test]
    #[serial]
    fn test_unified_loader_load_no_file() {
        clean_env();

        let dir = tempdir().unwrap();
        let loader = UnifiedConfigLoader::new()
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"));

        let result = loader.load();

        // Should return default config
        assert!(result.config.model_providers.contains_key("anthropic"));
        // Should be unconfigured (no keys found)
        assert!(result.needs_onboarding());
    }

    #[test]
    #[serial]
    fn test_unified_loader_load_with_file() {
        clean_env();

        let dir = tempdir().unwrap();
        let config_path = dir.path().join("sage_config.json");

        // Create a config file
        let config_content = r#"{
            "default_provider": "openai",
            "model_providers": {
                "openai": {
                    "model": "gpt-4",
                    "api_key": "test-key"
                }
            }
        }"#;
        std::fs::write(&config_path, config_content).unwrap();

        let loader = UnifiedConfigLoader::new()
            .with_config_file(&config_path)
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"));

        let result = loader.load();

        assert_eq!(result.config.default_provider, "openai");
        assert!(result.config_file.is_some());
        assert!(result.warnings.is_empty());
    }

    #[test]
    #[serial]
    fn test_unified_loader_load_nonexistent_file() {
        clean_env();

        let dir = tempdir().unwrap();
        let loader = UnifiedConfigLoader::new()
            .with_config_file("/nonexistent/path/config.json")
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"));

        let result = loader.load();

        // Should still return valid config (defaults)
        assert!(!result.config.model_providers.is_empty());
        // Should have warning about missing file
        assert!(!result.warnings.is_empty());
    }

    #[test]
    #[serial]
    fn test_unified_loader_cli_overrides() {
        clean_env();

        let dir = tempdir().unwrap();
        let loader = UnifiedConfigLoader::new()
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"))
            .with_cli_overrides(
                CliOverrides::new()
                    .with_provider("openai")
                    .with_max_steps(100)
                    .with_api_key("cli-api-key"),
            );

        let result = loader.load();

        assert_eq!(result.config.default_provider, "openai");
        assert_eq!(result.config.max_steps, Some(100));

        // CLI API key should be applied
        let openai_params = result.config.model_providers.get("openai").unwrap();
        assert_eq!(openai_params.api_key, Some("cli-api-key".to_string()));
    }

    #[test]
    #[serial]
    fn test_unified_loader_env_var_resolution() {
        clean_env();

        unsafe {
            env::set_var("ANTHROPIC_API_KEY", "env-anthropic-key");
        }

        let dir = tempdir().unwrap();
        let loader = UnifiedConfigLoader::new()
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"));

        let result = loader.load();

        // Should resolve API key from environment
        let anthropic_params = result.config.model_providers.get("anthropic").unwrap();
        assert_eq!(anthropic_params.api_key, Some("env-anthropic-key".to_string()));

        // Should be at least partial status
        assert!(result.is_ready());

        clean_env();
    }

    #[test]
    #[serial]
    fn test_unified_loader_project_config_discovery() {
        clean_env();

        let dir = tempdir().unwrap();

        // Create project-level config
        let config_content = r#"{
            "default_provider": "google",
            "model_providers": {
                "google": {
                    "model": "gemini-pro",
                    "api_key": "project-key"
                }
            }
        }"#;
        std::fs::write(dir.path().join("sage_config.json"), config_content).unwrap();

        let loader = UnifiedConfigLoader::new()
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"));

        let result = loader.load();

        assert_eq!(result.config.default_provider, "google");
        assert!(result.config_file.is_some());
    }

    #[test]
    fn test_unified_loader_default() {
        let loader = UnifiedConfigLoader::default();
        assert!(loader.config_file.is_none());
    }

    #[test]
    #[serial]
    fn test_load_config_unified_function() {
        clean_env();

        let result = load_config_unified(None);

        // Should return valid config with defaults
        assert!(!result.config.model_providers.is_empty());
    }

    #[test]
    #[serial]
    fn test_unified_loader_env_var_placeholder_replacement() {
        clean_env();

        let dir = tempdir().unwrap();
        let config_path = dir.path().join("sage_config.json");

        // Create a config with env var placeholder
        let config_content = r#"{
            "default_provider": "anthropic",
            "model_providers": {
                "anthropic": {
                    "model": "claude-3",
                    "api_key": "${ANTHROPIC_API_KEY}"
                }
            }
        }"#;
        std::fs::write(&config_path, config_content).unwrap();

        // Set the env var
        unsafe {
            env::set_var("ANTHROPIC_API_KEY", "resolved-key");
        }

        let loader = UnifiedConfigLoader::new()
            .with_config_file(&config_path)
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"));

        let result = loader.load();

        // The placeholder should be replaced with the actual env var value
        let anthropic_params = result.config.model_providers.get("anthropic").unwrap();
        assert_eq!(anthropic_params.api_key, Some("resolved-key".to_string()));

        clean_env();
    }
}
