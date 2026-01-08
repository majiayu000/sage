//! Logging configuration validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use std::collections::HashSet;

/// Validate logging configuration
pub fn validate_logging(config: &Config) -> SageResult<()> {
    // Validate log level
    let valid_levels: HashSet<&str> = ["trace", "debug", "info", "warn", "error"]
        .iter()
        .cloned()
        .collect();

    if !valid_levels.contains(config.logging.level.as_str()) {
        return Err(SageError::config(format!(
            "Invalid log level '{}'. Valid levels are: {:?}",
            config.logging.level, valid_levels
        )));
    }

    // Validate log format
    let valid_formats: HashSet<&str> = ["json", "pretty", "compact"].iter().cloned().collect();

    if !valid_formats.contains(config.logging.format.as_str()) {
        return Err(SageError::config(format!(
            "Invalid log format '{}'. Valid formats are: {:?}",
            config.logging.format, valid_formats
        )));
    }

    // Ensure at least one output is enabled
    if !config.logging.log_to_console && !config.logging.log_to_file {
        return Err(SageError::config(
            "At least one of log_to_console or log_to_file must be enabled",
        ));
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
    fn test_validate_logging_invalid_log_level() {
        let mut config = create_test_config();
        config.logging.level = "invalid".to_string();

        let result = validate_logging(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid log level")
        );
    }

    #[test]
    fn test_validate_logging_valid_log_levels() {
        let levels = vec!["trace", "debug", "info", "warn", "error"];

        for level in levels {
            let mut config = create_test_config();
            config.logging.level = level.to_string();
            assert!(validate_logging(&config).is_ok());
        }
    }

    #[test]
    fn test_validate_logging_invalid_format() {
        let mut config = create_test_config();
        config.logging.format = "xml".to_string(); // Invalid format

        let result = validate_logging(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid log format")
        );
    }

    #[test]
    fn test_validate_logging_valid_formats() {
        let formats = vec!["json", "pretty", "compact"];

        for format in formats {
            let mut config = create_test_config();
            config.logging.format = format.to_string();
            assert!(validate_logging(&config).is_ok());
        }
    }

    #[test]
    fn test_validate_logging_no_output_enabled() {
        let mut config = create_test_config();
        config.logging.log_to_console = false;
        config.logging.log_to_file = false;

        let result = validate_logging(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("At least one of log_to_console or log_to_file must be enabled")
        );
    }
}
