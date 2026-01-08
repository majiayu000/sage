//! Lakeview configuration validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};

/// Validate Lakeview configuration
pub fn validate_lakeview(config: &Config) -> SageResult<()> {
    if let Some(lakeview) = &config.lakeview_config {
        if lakeview.enabled {
            if lakeview.model_provider.is_empty() {
                return Err(SageError::config("Lakeview model provider cannot be empty"));
            }
            if lakeview.model_name.is_empty() {
                return Err(SageError::config("Lakeview model name cannot be empty"));
            }
        }
    }
    Ok(())
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
    fn test_validate_lakeview_disabled() {
        let config = create_test_config();
        // Lakeview is disabled, should pass validation
        assert!(validate_lakeview(&config).is_ok());
    }

    #[test]
    fn test_validate_lakeview_enabled_with_valid_config() {
        let mut config = create_test_config();
        config.lakeview_config = Some(crate::config::model::LakeviewConfig {
            model_provider: "openai".to_string(),
            model_name: "gpt-4".to_string(),
            endpoint: None,
            api_key: None,
            enabled: true,
        });

        assert!(validate_lakeview(&config).is_ok());
    }

    #[test]
    fn test_validate_lakeview_empty_provider() {
        let mut config = create_test_config();
        config.lakeview_config = Some(crate::config::model::LakeviewConfig {
            model_provider: "".to_string(), // Invalid
            model_name: "gpt-4".to_string(),
            endpoint: None,
            api_key: None,
            enabled: true,
        });

        let result = validate_lakeview(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Lakeview model provider cannot be empty")
        );
    }

    #[test]
    fn test_validate_lakeview_empty_model_name() {
        let mut config = create_test_config();
        config.lakeview_config = Some(crate::config::model::LakeviewConfig {
            model_provider: "openai".to_string(),
            model_name: "".to_string(), // Invalid
            endpoint: None,
            api_key: None,
            enabled: true,
        });

        let result = validate_lakeview(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Lakeview model name cannot be empty")
        );
    }
}
