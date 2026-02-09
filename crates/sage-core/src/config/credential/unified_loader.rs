//! Unified configuration loader that never fails
//!
//! This module provides a configuration loader that gracefully handles missing
//! or invalid configuration, returning usable defaults with status information.

use super::cli_overrides::CliOverrides;
use super::loaded_config::LoadedConfig;
use super::resolver::CredentialResolver;
use super::resolver_config::ResolverConfig;
use crate::config::model::Config;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Configuration loader that never fails
///
/// This loader:
/// 1. Attempts to load configuration from the specified file
/// 2. Falls back to environment variables
/// 3. Falls back to defaults
/// 4. Always returns a valid Config with status information
pub struct UnifiedConfigLoader {
    /// Path to the config file
    pub(crate) config_file: Option<PathBuf>,
    /// Working directory
    pub(crate) working_dir: PathBuf,
    /// Global config directory
    pub(crate) global_dir: PathBuf,
    /// CLI-provided overrides
    pub(crate) cli_overrides: CliOverrides,
}

impl UnifiedConfigLoader {
    /// Create a new unified config loader
    pub fn new() -> Self {
        Self {
            config_file: None,
            working_dir: std::env::current_dir().unwrap_or_default(),
            global_dir: dirs::home_dir().unwrap_or_default().join(".sage"),
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
            config_file_used = self.try_load_file(path, &mut config, &mut warnings);
        }

        // 2. Try default locations if no file specified or found
        if config_file_used.is_none() {
            config_file_used = self.try_default_locations(&mut config);
        }

        // 3. Apply CLI overrides
        self.apply_cli_overrides(&mut config);

        // 4. Resolve credentials and update config
        self.resolve_credentials(&mut config);

        // 5. Get configuration status
        let resolver = self.create_credential_resolver();
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

    /// Try to load from specified file
    fn try_load_file(
        &self,
        path: &Path,
        config: &mut Config,
        warnings: &mut Vec<String>,
    ) -> Option<PathBuf> {
        if path.exists() {
            match self.load_config_file(path) {
                Ok(file_config) => {
                    debug!("Loaded config from {}", path.display());
                    config.merge(file_config);
                    return Some(path.to_path_buf());
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
        None
    }

    /// Try loading from default locations
    fn try_default_locations(&self, config: &mut Config) -> Option<PathBuf> {
        // Try project-level config
        let project_config = self.working_dir.join("sage_config.json");
        if project_config.exists() {
            if let Ok(file_config) = self.load_config_file(&project_config) {
                debug!("Loaded project config from {}", project_config.display());
                config.merge(file_config);
                return Some(project_config);
            }
        }

        // Try global config
        let global_config = self.global_dir.join("config.json");
        if global_config.exists() {
            if let Ok(file_config) = self.load_config_file(&global_config) {
                debug!("Loaded global config from {}", global_config.display());
                config.merge(file_config);
                return Some(global_config);
            }
        }

        None
    }

    /// Load a config file
    fn load_config_file(&self, path: &Path) -> Result<Config, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Apply CLI overrides to the config
    fn apply_cli_overrides(&self, config: &mut Config) {
        if let Some(ref provider) = self.cli_overrides.provider {
            config.default_provider = provider.clone();
        }

        if let Some(max_steps) = self.cli_overrides.max_steps {
            config.max_steps = Some(max_steps);
        }

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

    /// Resolve credentials and update config
    fn resolve_credentials(&self, config: &mut Config) {
        let resolver = self.create_credential_resolver();
        let credentials = resolver.resolve_all();

        for cred in credentials.iter() {
            if let Some(value) = cred.value() {
                if let Some(params) = config.model_providers.get_mut(&cred.provider) {
                    let should_update = params.api_key.is_none()
                        || params.api_key.as_deref() == Some("")
                        || params
                            .api_key
                            .as_deref()
                            .is_some_and(|k| k.starts_with("${"));
                    if should_update {
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
    }

    /// Create a credential resolver based on current settings
    fn create_credential_resolver(&self) -> CredentialResolver {
        let mut config = ResolverConfig::new(&self.working_dir).with_global_dir(&self.global_dir);

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
mod tests;
