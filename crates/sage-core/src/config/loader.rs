//! Configuration loading and management

use crate::config::model::{Config, ModelParameters};
use crate::error::{SageError, SageResult};
use anyhow::Context;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Source of configuration data
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Configuration from a file
    File(PathBuf),
    /// Configuration from environment variables
    Environment,
    /// Configuration from command line arguments
    CommandLine(HashMap<String, String>),
    /// Default configuration
    Default,
}

/// Configuration loader with support for multiple sources
pub struct ConfigLoader {
    sources: Vec<ConfigSource>,
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
            let source_config = self.load_from_source(source)?;
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

    /// Load configuration from a specific source
    fn load_from_source(&self, source: &ConfigSource) -> SageResult<Config> {
        match source {
            ConfigSource::File(path) => {
                tracing::debug!("Loading config from file: {}", path.display());
                let config = self.load_from_file(path)?;
                tracing::debug!("File config provider: {}", config.default_provider);
                Ok(config)
            }
            ConfigSource::Environment => {
                tracing::debug!("Loading config from environment");
                let config = self.load_from_env()?;
                tracing::debug!("Env config provider: {}", config.default_provider);
                Ok(config)
            }
            ConfigSource::CommandLine(args) => {
                tracing::debug!("Loading config from command line");
                self.load_from_args(args)
            }
            ConfigSource::Default => {
                tracing::debug!("Loading default config");
                let config = Config::default();
                tracing::debug!("Default config provider: {}", config.default_provider);
                Ok(config)
            }
        }
    }

    /// Load configuration from a file
    fn load_from_file(&self, path: &Path) -> SageResult<Config> {
        if !path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(path)
            .map_err(|e| SageError::config(format!("Failed to read config file: {}", e)))
            .with_context(|| format!("failed to read configuration from '{}'", path.display()))?;

        let config: Config = match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => toml::from_str(&content)
                .map_err(|e| SageError::config(format!("Failed to parse TOML config: {}", e)))
                .with_context(|| format!("failed to deserialize TOML configuration from '{}'", path.display()))?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .map_err(|e| SageError::config(format!("Failed to parse YAML config: {}", e)))
                .with_context(|| format!("failed to deserialize YAML configuration from '{}'", path.display()))?,
            _ => serde_json::from_str(&content)
                .map_err(|e| SageError::config(format!("Failed to parse JSON config: {}", e)))
                .with_context(|| format!("failed to deserialize JSON configuration from '{}'", path.display()))?,
        };

        Ok(config)
    }

    /// Load configuration from environment variables
    fn load_from_env(&self) -> SageResult<Config> {
        let mut config = Config {
            default_provider: String::new(), // Don't set default here
            max_steps: None,                 // None = unlimited
            total_token_budget: None,
            model_providers: HashMap::new(),
            lakeview_config: None,
            enable_lakeview: false,
            working_directory: None,
            tools: crate::config::model::ToolConfig {
                enabled_tools: Vec::new(),
                tool_settings: HashMap::new(),
                max_execution_time: 0,
                allow_parallel_execution: false,
            },
            logging: crate::config::model::LoggingConfig::default(),
            trajectory: crate::config::model::TrajectoryConfig::default(),
            mcp: crate::config::model::McpConfig::default(),
        };

        // Load provider settings
        if let Ok(provider) = env::var("SAGE_DEFAULT_PROVIDER") {
            config.default_provider = provider;
        }

        if let Ok(max_steps_str) = env::var("SAGE_MAX_STEPS") {
            let max_steps: u32 = max_steps_str
                .parse()
                .map_err(|_| SageError::config("Invalid SAGE_MAX_STEPS value"))?;
            config.max_steps = Some(max_steps);
        }

        // Load model parameters for different providers
        self.load_provider_from_env(&mut config, "openai", "OPENAI")?;
        self.load_provider_from_env(&mut config, "anthropic", "ANTHROPIC")?;
        self.load_provider_from_env(&mut config, "google", "GOOGLE")?;
        self.load_provider_from_env(&mut config, "ollama", "OLLAMA")?;

        // Load working directory
        if let Ok(working_dir) = env::var("SAGE_WORKING_DIR") {
            config.working_directory = Some(PathBuf::from(working_dir));
        }

        // Load Lakeview settings
        if let Ok(enable_lakeview) = env::var("SAGE_ENABLE_LAKEVIEW") {
            config.enable_lakeview = enable_lakeview.parse().unwrap_or(false);
        }

        Ok(config)
    }

    /// Load provider configuration from environment variables
    fn load_provider_from_env(
        &self,
        config: &mut Config,
        provider: &str,
        env_prefix: &str,
    ) -> SageResult<()> {
        let mut params = ModelParameters::default();
        let mut has_config = false;

        // API Key
        if let Ok(api_key) = env::var(format!("{}_API_KEY", env_prefix)) {
            params.api_key = Some(api_key);
            has_config = true;
        }

        // Model
        if let Ok(model) = env::var(format!("{}_MODEL", env_prefix)) {
            params.model = model;
            has_config = true;
        }

        // Base URL
        if let Ok(base_url) = env::var(format!("{}_BASE_URL", env_prefix)) {
            params.base_url = Some(base_url);
            has_config = true;
        }

        // Temperature
        if let Ok(temp) = env::var(format!("{}_TEMPERATURE", env_prefix)) {
            params.temperature = Some(temp.parse()
                .map_err(|_| SageError::config(format!("Invalid {}_TEMPERATURE value", env_prefix)))
                .with_context(|| format!("failed to parse temperature value '{}' for provider '{}'", temp, provider))?);
            has_config = true;
        }

        // Max tokens
        if let Ok(max_tokens) = env::var(format!("{}_MAX_TOKENS", env_prefix)) {
            params.max_tokens = Some(max_tokens.parse()
                .map_err(|_| SageError::config(format!("Invalid {}_MAX_TOKENS value", env_prefix)))
                .with_context(|| format!("failed to parse max_tokens value '{}' for provider '{}'", max_tokens, provider))?);
            has_config = true;
        }

        if has_config {
            config.model_providers.insert(provider.to_string(), params);
        }

        Ok(())
    }

    /// Load configuration from command line arguments
    fn load_from_args(&self, args: &HashMap<String, String>) -> SageResult<Config> {
        let mut config = Config {
            default_provider: String::new(), // Don't set default here
            max_steps: None,                 // None = unlimited
            total_token_budget: None,
            model_providers: HashMap::new(),
            lakeview_config: None,
            enable_lakeview: false,
            working_directory: None,
            tools: crate::config::model::ToolConfig {
                enabled_tools: Vec::new(),
                tool_settings: HashMap::new(),
                max_execution_time: 0,
                allow_parallel_execution: false,
            },
            logging: crate::config::model::LoggingConfig::default(),
            trajectory: crate::config::model::TrajectoryConfig::default(),
            mcp: crate::config::model::McpConfig::default(),
        };

        if let Some(provider) = args.get("provider") {
            config.default_provider = provider.clone();
        }

        if let Some(model) = args.get("model") {
            // Update the model for the current provider
            let provider = config.default_provider.clone();
            let mut params = config
                .model_providers
                .get(&provider)
                .cloned()
                .unwrap_or_default();
            params.model = model.clone();
            config.model_providers.insert(provider, params);
        }

        if let Some(api_key) = args.get("api_key") {
            let provider = config.default_provider.clone();
            let mut params = config
                .model_providers
                .get(&provider)
                .cloned()
                .unwrap_or_default();
            params.api_key = Some(api_key.clone());
            config.model_providers.insert(provider, params);
        }

        if let Some(base_url) = args.get("model_base_url") {
            let provider = config.default_provider.clone();
            let mut params = config
                .model_providers
                .get(&provider)
                .cloned()
                .unwrap_or_default();
            params.base_url = Some(base_url.clone());
            config.model_providers.insert(provider, params);
        }

        if let Some(max_steps_str) = args.get("max_steps") {
            let max_steps: u32 = max_steps_str
                .parse()
                .map_err(|_| SageError::config("Invalid max_steps value"))
                .with_context(|| format!("failed to parse max_steps value '{}' from command line arguments", max_steps_str))?;
            config.max_steps = Some(max_steps);
        }

        if let Some(working_dir) = args.get("working_dir") {
            config.working_directory = Some(PathBuf::from(working_dir));
        }

        Ok(config)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to load configuration with default sources
pub fn load_config() -> SageResult<Config> {
    ConfigLoader::new()
        .with_defaults()
        .with_file("sage_config.json")
        .with_file("sage_config.toml")
        .with_env()
        .load()
}

/// Load configuration with custom file path
pub fn load_config_from_file<P: AsRef<Path>>(path: P) -> SageResult<Config> {
    ConfigLoader::new()
        .with_defaults()
        .with_file(path)
        .with_env()
        .load()
}

/// Load configuration with command line overrides
pub fn load_config_with_overrides(
    config_file: Option<&str>,
    overrides: HashMap<String, String>,
) -> SageResult<Config> {
    let mut loader = ConfigLoader::new().with_defaults().with_env();

    if let Some(file) = config_file {
        loader = loader.with_file(file);
    } else {
        loader = loader
            .with_file("sage_config.json")
            .with_file("sage_config.toml");
    }

    loader.with_args(overrides).load()
}
