//! Configuration validation
//!
//! This module provides comprehensive validation for all configuration aspects including
//! providers, models, limits, paths, tools, and logging settings.

mod lakeview;
mod limits;
mod logging;
mod model;
mod paths;
mod provider;
mod tools;

// Re-export validation functions to maintain backward compatibility
pub use lakeview::validate_lakeview;
pub use limits::validate_limits;
pub use logging::validate_logging;
pub use model::validate_models;
pub use paths::validate_paths;
pub use provider::validate_providers;
pub use tools::validate_tools;

use crate::config::model::Config;
use crate::error::SageResult;

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a complete configuration
    ///
    /// This performs comprehensive validation of all configuration aspects:
    /// - Providers: Checks provider names and default provider existence
    /// - Models: Validates model parameters and API keys
    /// - Limits: Validates execution limits and timeouts
    /// - Paths: Checks working directory and log file paths
    /// - Tools: Validates enabled tools and requirements
    ///
    /// # Errors
    ///
    /// Returns an error if any validation check fails.
    pub fn validate(config: &Config) -> SageResult<()> {
        validate_providers(config)?;
        validate_models(config)?;
        validate_limits(config)?;
        validate_paths(config)?;
        validate_tools(config)?;
        Ok(())
    }

    /// Validate provider configuration
    pub fn validate_providers(config: &Config) -> SageResult<()> {
        validate_providers(config)
    }

    /// Validate model configurations
    pub fn validate_models(config: &Config) -> SageResult<()> {
        validate_models(config)
    }

    /// Validate limits and constraints
    pub fn validate_limits(config: &Config) -> SageResult<()> {
        validate_limits(config)
    }

    /// Validate file paths
    pub fn validate_paths(config: &Config) -> SageResult<()> {
        validate_paths(config)
    }

    /// Validate tool configuration
    pub fn validate_tools(config: &Config) -> SageResult<()> {
        validate_tools(config)
    }

    /// Validate logging configuration
    pub fn validate_logging(config: &Config) -> SageResult<()> {
        validate_logging(config)
    }

    /// Validate Lakeview configuration
    pub fn validate_lakeview(config: &Config) -> SageResult<()> {
        validate_lakeview(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{
        LoggingConfig, McpConfig, ModelParameters, ToolConfig, TrajectoryConfig,
    };
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        let mut model_providers = HashMap::new();
        model_providers.insert(
            "anthropic".to_string(),
            ModelParameters {
                model: "claude-3".to_string(),
                api_key: Some("test_key".to_string()),
                max_tokens: Some(4096),
                temperature: Some(0.7),
                top_p: Some(0.9),
                top_k: Some(40),
                parallel_tool_calls: Some(true),
                max_retries: Some(3),
                base_url: Some("https://api.anthropic.com".to_string()),
                api_version: None,
                stop_sequences: None,
            },
        );

        Config {
            default_provider: "anthropic".to_string(),
            max_steps: Some(50),
            total_token_budget: Some(100000),
            model_providers,
            lakeview_config: None,
            enable_lakeview: false,
            working_directory: Some(std::env::temp_dir()),
            tools: ToolConfig {
                tool_settings: std::collections::HashMap::new(),
                max_execution_time: 300,
                allow_parallel_execution: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                log_to_console: true,
                log_to_file: false,
                log_file: None,
            },
            trajectory: TrajectoryConfig::default(),
            mcp: McpConfig::default(),
        }
    }

    #[test]
    fn test_validate_full_config_success() {
        let config = create_test_config();
        assert!(ConfigValidator::validate(&config).is_ok());
    }

    #[test]
    fn test_validate_full_config_multiple_errors() {
        let mut config = create_test_config();

        // Introduce multiple errors
        config.default_provider = "nonexistent".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        // Should catch the first error (missing default provider)
    }
}
