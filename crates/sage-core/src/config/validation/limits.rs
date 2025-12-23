//! Configuration limits validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};

/// Validate limits and constraints
pub fn validate_limits(config: &Config) -> SageResult<()> {
    // Validate max_steps (if set; None means unlimited)
    if let Some(max_steps) = config.max_steps {
        if max_steps == 0 {
            return Err(SageError::config(
                "Max steps must be greater than 0 (use None for unlimited)",
            ));
        }
        if max_steps > 1000 {
            return Err(SageError::config(format!(
                "Max steps seems too large: {}. Consider using a smaller value or None for unlimited",
                max_steps
            )));
        }
    }
    // Note: None (unlimited) is valid and means no step limit

    // Validate tool execution time
    if config.tools.max_execution_time == 0 {
        return Err(SageError::config(
            "Tool max execution time must be greater than 0",
        ));
    }
    if config.tools.max_execution_time > 3600 {
        return Err(SageError::config(format!(
            "Tool max execution time seems too large: {} seconds",
            config.tools.max_execution_time
        )));
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
                enabled_tools: vec!["task_done".to_string(), "bash".to_string()],
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
    fn test_validate_limits_zero_max_steps() {
        let mut config = create_test_config();
        config.max_steps = Some(0);

        let result = validate_limits(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Max steps must be greater than 0")
        );
    }

    #[test]
    fn test_validate_limits_excessive_max_steps() {
        let mut config = create_test_config();
        config.max_steps = Some(2000);

        let result = validate_limits(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Max steps seems too large")
        );
    }

    #[test]
    fn test_validate_limits_none_max_steps_allowed() {
        let mut config = create_test_config();
        config.max_steps = None; // Unlimited should be valid

        assert!(validate_limits(&config).is_ok());
    }

    #[test]
    fn test_validate_limits_zero_tool_execution_time() {
        let mut config = create_test_config();
        config.tools.max_execution_time = 0;

        let result = validate_limits(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Tool max execution time must be greater than 0")
        );
    }

    #[test]
    fn test_validate_limits_excessive_tool_execution_time() {
        let mut config = create_test_config();
        config.tools.max_execution_time = 5000; // Too long

        let result = validate_limits(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Tool max execution time seems too large")
        );
    }
}
