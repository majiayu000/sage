//! Tool configuration validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use std::collections::HashSet;

/// Validate tool configuration
pub fn validate_tools(config: &Config) -> SageResult<()> {
    // Validate enabled tools
    let valid_tools: HashSet<&str> = [
        "str_replace_based_edit_tool",
        "sequentialthinking",
        "json_edit_tool",
        "task_done",
        "bash",
    ]
    .iter()
    .cloned()
    .collect();

    for tool in &config.tools.enabled_tools {
        if !valid_tools.contains(tool.as_str()) && !tool.starts_with("custom_") {
            return Err(SageError::config(format!(
                "Unknown tool '{}'. Valid tools are: {:?}",
                tool, valid_tools
            )));
        }
    }

    // Ensure task_done tool is always enabled
    if !config
        .tools
        .enabled_tools
        .contains(&"task_done".to_string())
    {
        return Err(SageError::config(
            "The 'task_done' tool must be enabled for proper agent operation",
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
    fn test_validate_tools_unknown_tool() {
        let mut config = create_test_config();
        config.tools.enabled_tools.push("unknown_tool".to_string());

        let result = validate_tools(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));
    }

    #[test]
    fn test_validate_tools_custom_tool_allowed() {
        let mut config = create_test_config();
        config
            .tools
            .enabled_tools
            .push("custom_my_tool".to_string());

        // Custom tools with custom_ prefix should be allowed
        assert!(validate_tools(&config).is_ok());
    }

    #[test]
    fn test_validate_tools_task_done_required() {
        let mut config = create_test_config();
        config.tools.enabled_tools = vec!["bash".to_string()]; // Missing task_done

        let result = validate_tools(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task_done"));
    }
}
