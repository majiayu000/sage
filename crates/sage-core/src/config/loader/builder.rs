//! Configuration loader builder

use super::loading::load_from_source;
use super::types::ConfigSource;
use crate::config::model::Config;
use crate::error::SageResult;
use std::collections::HashMap;
use std::path::Path;

/// Configuration loader with support for multiple sources
pub struct ConfigLoader {
    pub(super) sources: Vec<ConfigSource>,
}

impl ConfigLoader {
    /// Create a new config loader
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add a configuration source
    pub fn add_source(mut self, source: ConfigSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Add a file source
    pub fn with_file<P: AsRef<Path>>(self, path: P) -> Self {
        self.add_source(ConfigSource::File(path.as_ref().to_path_buf()))
    }

    /// Add environment variables source
    pub fn with_env(self) -> Self {
        self.add_source(ConfigSource::Environment)
    }

    /// Add command line arguments source
    pub fn with_args(self, args: HashMap<String, String>) -> Self {
        self.add_source(ConfigSource::CommandLine(args))
    }

    /// Add default configuration source
    pub fn with_defaults(self) -> Self {
        self.add_source(ConfigSource::Default)
    }

    /// Load configuration from all sources
    pub fn load(self) -> SageResult<Config> {
        let mut config = Config::default();
        tracing::debug!("Initial config provider: {}", config.default_provider);

        for source in &self.sources {
            let source_config = load_from_source(source)?;
            tracing::debug!(
                "Before merge - config provider: {}",
                config.default_provider
            );
            config.merge(source_config);
            tracing::debug!("After merge - config provider: {}", config.default_provider);
        }

        config.validate()?;
        Ok(config)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
