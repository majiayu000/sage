//! SDK constructors

use crate::client::SageAgentSdk;
use sage_core::{
    config::{load_config_with_overrides, model::Config},
    error::SageResult,
};
use std::collections::HashMap;

impl SageAgentSdk {
    /// Create a new SDK instance with default configuration.
    ///
    /// Loads configuration from the default search paths, applying environment
    /// variable substitutions.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration file cannot be found or parsed
    /// - Required environment variables are missing
    /// - Configuration validation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new()?;
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn new() -> SageResult<Self> {
        let config = load_config_with_overrides(None, HashMap::new())?;
        Ok(Self { config })
    }

    /// Create SDK instance with custom configuration.
    ///
    /// Use this when you have already constructed a `Config` object programmatically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::{SageAgentSdk, Config};
    ///
    /// let config = Config::default();
    /// let sdk = SageAgentSdk::with_config(config);
    /// ```
    pub fn with_config(config: Config) -> Self {
        Self { config }
    }

    /// Create SDK instance with configuration file.
    ///
    /// Loads configuration from the specified file path, applying environment
    /// variable substitutions.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File does not exist or cannot be read
    /// - File contains invalid JSON/TOML
    /// - Configuration validation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::with_config_file("config/sage.json")?;
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn with_config_file<P: AsRef<std::path::Path>>(config_file: P) -> SageResult<Self> {
        let config_path = config_file.as_ref();
        tracing::info!("Loading SDK config from: {}", config_path.display());

        let path_str = config_file
            .as_ref()
            .to_str()
            .ok_or_else(|| sage_core::error::SageError::config("Config file path contains invalid UTF-8"))?;
        let config = load_config_with_overrides(Some(path_str), HashMap::new())?;

        tracing::info!(
            "SDK config loaded - provider: {}, model: {}",
            config.get_default_provider(),
            config
                .default_model_parameters()
                .map(|p| p.model.clone())
                .unwrap_or_else(|_| "unknown".to_string())
        );

        Ok(Self { config })
    }
}

impl Default for SageAgentSdk {
    /// Creates SDK with default configuration.
    ///
    /// # Panics
    ///
    /// Panics if the default configuration cannot be loaded. Use `SageAgentSdk::new()`
    /// for fallible construction.
    fn default() -> Self {
        Self::new().expect("Failed to load default SDK configuration")
    }
}
