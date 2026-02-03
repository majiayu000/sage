//! Main configuration for Sage Agent

use crate::config::model::{
    LakeviewConfig, LoggingConfig, McpConfig, ModelParameters, ToolConfig, TrajectoryConfig,
};
use crate::config::provider_defaults::create_default_providers;
use crate::error::{SageError, SageResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration for Sage Agent
///
/// All fields support serde(default) to allow partial configuration files
/// to be merged with defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Default LLM provider to use
    pub default_provider: String,
    /// Maximum number of execution steps (None = unlimited)
    pub max_steps: Option<u32>,
    /// Total token budget across all steps (input + output)
    /// When exceeded, agent will stop with a budget exceeded error
    pub total_token_budget: Option<u64>,
    /// Model parameters for each provider
    pub model_providers: HashMap<String, ModelParameters>,
    /// Lakeview configuration
    pub lakeview_config: Option<LakeviewConfig>,
    /// Whether to enable Lakeview
    pub enable_lakeview: bool,
    /// Working directory for the agent
    pub working_directory: Option<PathBuf>,
    /// Tool configuration
    pub tools: ToolConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Trajectory configuration
    pub trajectory: TrajectoryConfig,
    /// MCP (Model Context Protocol) configuration
    pub mcp: McpConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: "anthropic".to_string(),
            max_steps: None,          // None = unlimited steps
            total_token_budget: None, // No limit by default
            model_providers: create_default_providers(),
            lakeview_config: None,
            enable_lakeview: true, // Python version defaults to true
            working_directory: None,
            tools: ToolConfig::default(),
            logging: LoggingConfig::default(),
            trajectory: TrajectoryConfig::default(),
            mcp: McpConfig::default(),
        }
    }
}

impl Config {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the model parameters for the default provider
    pub fn default_model_parameters(&self) -> SageResult<&ModelParameters> {
        self.model_providers
            .get(&self.default_provider)
            .ok_or_else(|| {
                SageError::config(format!(
                    "No model parameters found for default provider: {}",
                    self.default_provider
                ))
            })
    }

    /// Get the default provider as string
    pub fn get_default_provider(&self) -> &str {
        &self.default_provider
    }

    /// Add or update model parameters for a provider
    pub fn set_model_parameters(&mut self, provider: String, params: ModelParameters) {
        self.model_providers.insert(provider, params);
    }

    /// Set the default provider
    pub fn set_default_provider(&mut self, provider: String) -> SageResult<()> {
        if !self.model_providers.contains_key(&provider) {
            return Err(SageError::config(format!(
                "Provider '{}' not found in model_providers",
                provider
            )));
        }
        self.default_provider = provider;
        Ok(())
    }

    /// Set the model for the default provider
    pub fn set_default_model(&mut self, model: String) {
        if let Some(params) = self.model_providers.get_mut(&self.default_provider) {
            params.model = model;
        }
    }

    /// Validate the entire configuration
    pub fn validate(&self) -> SageResult<()> {
        // Validate default provider exists
        if !self.model_providers.contains_key(&self.default_provider) {
            return Err(SageError::config(format!(
                "Default provider '{}' not found in model_providers",
                self.default_provider
            )));
        }

        // Validate max steps (if set)
        if let Some(max_steps) = self.max_steps {
            if max_steps == 0 {
                return Err(SageError::config(
                    "Max steps must be greater than 0 (use None for unlimited)",
                ));
            }
        }

        // Validate all model parameters
        for (provider, params) in &self.model_providers {
            params.validate().map_err(|e| {
                SageError::config(format!(
                    "Invalid parameters for provider '{}': {}",
                    provider, e
                ))
            })?;
        }

        // Validate working directory if set
        if let Some(working_dir) = &self.working_directory {
            if !working_dir.exists() {
                return Err(SageError::config(format!(
                    "Working directory does not exist: {}",
                    working_dir.display()
                )));
            }
        }

        Ok(())
    }

    /// Merge with another config (other takes precedence)
    ///
    /// Uses deep merge for model providers - field-level overrides are applied
    /// rather than replacing the entire provider config. This allows users to
    /// specify only the fields they want to change in their config file.
    ///
    /// # Example
    ///
    /// If base config has anthropic with max_tokens=4096, temperature=0.7
    /// and override config has anthropic with max_tokens=8192,
    /// the result will have max_tokens=8192 but temperature=0.7 (preserved).
    pub fn merge(&mut self, other: Config) {
        if !other.default_provider.is_empty() {
            self.default_provider = other.default_provider;
        }

        // Merge max_steps if other has a value set
        if other.max_steps.is_some() {
            self.max_steps = other.max_steps;
        }

        // Merge total_token_budget if other has a value set
        if other.total_token_budget.is_some() {
            self.total_token_budget = other.total_token_budget;
        }

        // Deep merge model providers - field-level override instead of full replacement
        for (provider, other_params) in other.model_providers {
            if let Some(existing_params) = self.model_providers.get_mut(&provider) {
                // Deep merge: existing params are updated with other's non-None fields
                existing_params.merge(other_params);
            } else {
                // Provider doesn't exist in base, add it
                self.model_providers.insert(provider, other_params);
            }
        }

        if other.lakeview_config.is_some() {
            self.lakeview_config = other.lakeview_config;
        }

        self.enable_lakeview = other.enable_lakeview;

        if other.working_directory.is_some() {
            self.working_directory = other.working_directory;
        }

        self.tools.merge(other.tools);
        self.logging.merge(other.logging);
        self.mcp.merge(other.mcp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, None);
        assert!(config.model_providers.contains_key("anthropic"));
        assert!(config.model_providers.contains_key("openai"));
        assert!(config.model_providers.contains_key("google"));
    }

    #[test]
    fn test_config_new() {
        let config = Config::new();
        assert_eq!(config.default_provider, "anthropic");
    }

    #[test]
    fn test_config_default_model_parameters() {
        let config = Config::default();
        let params = config.default_model_parameters().unwrap();
        assert_eq!(params.model, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn test_config_default_model_parameters_missing() {
        let mut config = Config::default();
        config.default_provider = "nonexistent".to_string();
        assert!(config.default_model_parameters().is_err());
    }

    #[test]
    fn test_config_get_default_provider() {
        let config = Config::default();
        assert_eq!(config.get_default_provider(), "anthropic");
    }

    #[test]
    fn test_config_set_model_parameters() {
        let mut config = Config::default();
        let params = ModelParameters {
            model: "new-model".to_string(),
            ..Default::default()
        };
        config.set_model_parameters("custom".to_string(), params);
        assert!(config.model_providers.contains_key("custom"));
    }

    #[test]
    fn test_config_set_default_provider_success() {
        let mut config = Config::default();
        assert!(config.set_default_provider("openai".to_string()).is_ok());
        assert_eq!(config.default_provider, "openai");
    }

    #[test]
    fn test_config_set_default_provider_not_found() {
        let mut config = Config::default();
        assert!(
            config
                .set_default_provider("nonexistent".to_string())
                .is_err()
        );
    }

    #[test]
    fn test_config_validate_success() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_missing_default_provider() {
        let mut config = Config::default();
        config.default_provider = "nonexistent".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_max_steps() {
        let mut config = Config::default();
        config.max_steps = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_model_parameters() {
        let mut config = Config::default();
        let invalid_params = ModelParameters {
            model: "".to_string(), // Empty model name
            ..Default::default()
        };
        config
            .model_providers
            .insert("anthropic".to_string(), invalid_params);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = Config::default();
        let mut config2 = Config::default();
        config2.default_provider = "openai".to_string();
        config2.max_steps = Some(100);

        config1.merge(config2);
        assert_eq!(config1.default_provider, "openai");
        assert_eq!(config1.max_steps, Some(100));
    }

    #[test]
    fn test_config_merge_empty_provider() {
        let mut config1 = Config::default();
        let mut config2 = Config::default();
        config2.default_provider = "".to_string();

        config1.merge(config2);
        // Empty provider should not override
        assert_eq!(config1.default_provider, "anthropic");
    }

    #[test]
    fn test_config_merge_deep_model_params() {
        let mut config1 = Config::default();
        // Base config has default anthropic settings

        // Override config only specifies max_tokens for anthropic
        let mut config2 = Config::default();
        config2.model_providers.clear();
        config2.model_providers.insert(
            "anthropic".to_string(),
            ModelParameters {
                model: "".to_string(),   // Empty = don't override
                max_tokens: Some(16384), // Override this
                temperature: None,       // Keep base
                ..Default::default()
            },
        );

        config1.merge(config2);

        // Check that deep merge worked
        let anthropic = config1.model_providers.get("anthropic").unwrap();
        // max_tokens should be overridden
        assert_eq!(anthropic.max_tokens, Some(16384));
        // temperature should be preserved from base (not None!)
        assert!(anthropic.temperature.is_some());
        // model should be preserved (claude-sonnet-4-xxx from defaults)
        assert!(anthropic.model.contains("claude"));
    }

    #[test]
    fn test_config_merge_new_provider() {
        let mut config1 = Config::default();

        // Add a new provider that doesn't exist in base
        let mut config2 = Config::default();
        config2.model_providers.clear();
        config2.model_providers.insert(
            "custom".to_string(),
            ModelParameters {
                model: "custom-model".to_string(),
                max_tokens: Some(8192),
                ..Default::default()
            },
        );

        config1.merge(config2);

        // New provider should be added
        assert!(config1.model_providers.contains_key("custom"));
        let custom = config1.model_providers.get("custom").unwrap();
        assert_eq!(custom.model, "custom-model");
        assert_eq!(custom.max_tokens, Some(8192));

        // Existing providers should still exist
        assert!(config1.model_providers.contains_key("anthropic"));
    }

    #[test]
    fn test_config_debug() {
        let config = Config::default();
        let debug_string = format!("{:?}", config);
        assert!(debug_string.contains("Config"));
    }

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(config.default_provider, cloned.default_provider);
    }
}
