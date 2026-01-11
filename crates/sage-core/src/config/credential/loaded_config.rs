//! Loaded configuration result type
//!
//! This module provides the LoadedConfig type which wraps a configuration
//! with status information about how it was loaded.

use super::status::ConfigStatusReport;
use crate::config::model::Config;
use std::path::PathBuf;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
