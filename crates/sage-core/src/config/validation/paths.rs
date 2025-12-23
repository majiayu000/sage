//! File path validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};

/// Validate file paths
pub fn validate_paths(config: &Config) -> SageResult<()> {
    // Validate working directory
    if let Some(working_dir) = &config.working_directory {
        if !working_dir.exists() {
            return Err(SageError::config(format!(
                "Working directory does not exist: {}",
                working_dir.display()
            )));
        }
        if !working_dir.is_dir() {
            return Err(SageError::config(format!(
                "Working directory is not a directory: {}",
                working_dir.display()
            )));
        }
    }

    // Validate log file path
    if let Some(log_file) = &config.logging.log_file {
        if let Some(parent) = log_file.parent() {
            if !parent.exists() {
                return Err(SageError::config(format!(
                    "Log file directory does not exist: {}",
                    parent.display()
                )));
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
    fn test_validate_paths_nonexistent_working_directory() {
        let mut config = create_test_config();
        config.working_directory = Some(std::path::PathBuf::from(
            "/nonexistent/path/that/does/not/exist",
        ));

        let result = validate_paths(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Working directory does not exist")
        );
    }

    #[test]
    fn test_validate_paths_working_directory_not_a_directory() {
        let mut config = create_test_config();

        // Create a temporary file (not directory)
        let temp_file = std::env::temp_dir().join("test_file.txt");
        std::fs::write(&temp_file, "test").unwrap();
        config.working_directory = Some(temp_file.clone());

        let result = validate_paths(&config);

        // Clean up
        std::fs::remove_file(&temp_file).ok();

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Working directory is not a directory")
        );
    }

    #[test]
    fn test_validate_paths_log_file_directory_not_exist() {
        let mut config = create_test_config();
        config.logging.log_file = Some(std::path::PathBuf::from("/nonexistent/dir/log.txt"));

        let result = validate_paths(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Log file directory does not exist")
        );
    }
}
