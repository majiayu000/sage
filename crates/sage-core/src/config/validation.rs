//! Configuration validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use std::collections::HashSet;

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a complete configuration
    pub fn validate(config: &Config) -> SageResult<()> {
        Self::validate_providers(config)?;
        Self::validate_models(config)?;
        Self::validate_limits(config)?;
        Self::validate_paths(config)?;
        Self::validate_tools(config)?;
        Ok(())
    }

    /// Validate provider configuration
    fn validate_providers(config: &Config) -> SageResult<()> {
        // Check that default provider exists
        if !config
            .model_providers
            .contains_key(&config.default_provider)
        {
            return Err(SageError::config(format!(
                "Default provider '{}' not found in model_providers",
                config.default_provider
            )));
        }

        // Validate provider names
        // All providers from LLMProvider enum in llm/providers.rs
        let valid_providers: HashSet<&str> = [
            "openai",     // OpenAI (GPT models)
            "anthropic",  // Anthropic (Claude models)
            "google",     // Google (Gemini models)
            "azure",      // Azure OpenAI
            "openrouter", // OpenRouter
            "doubao",     // Doubao
            "ollama",     // Ollama (local models)
            "glm",        // GLM (Zhipu AI)
            "zhipu",      // Alias for GLM
        ]
        .iter()
        .cloned()
        .collect();

        for provider in config.model_providers.keys() {
            if !valid_providers.contains(provider.as_str()) && !provider.starts_with("custom_") {
                return Err(SageError::config(format!(
                    "Unknown provider '{}'. Valid providers are: {:?}",
                    provider, valid_providers
                )));
            }
        }

        Ok(())
    }

    /// Validate model configurations
    fn validate_models(config: &Config) -> SageResult<()> {
        for (provider, params) in &config.model_providers {
            // Validate model name
            if params.model.is_empty() {
                return Err(SageError::config(format!(
                    "Model name cannot be empty for provider '{}'",
                    provider
                )));
            }

            // Validate temperature
            if let Some(temp) = params.temperature {
                if !(0.0..=2.0).contains(&temp) {
                    return Err(SageError::config(format!(
                        "Temperature must be between 0.0 and 2.0 for provider '{}', got {}",
                        provider, temp
                    )));
                }
            }

            // Validate top_p
            if let Some(top_p) = params.top_p {
                if !(0.0..=1.0).contains(&top_p) {
                    return Err(SageError::config(format!(
                        "Top-p must be between 0.0 and 1.0 for provider '{}', got {}",
                        provider, top_p
                    )));
                }
            }

            // Validate max_tokens
            if let Some(max_tokens) = params.max_tokens {
                if max_tokens == 0 {
                    return Err(SageError::config(format!(
                        "Max tokens must be greater than 0 for provider '{}'",
                        provider
                    )));
                }
                if max_tokens > 1_000_000 {
                    return Err(SageError::config(format!(
                        "Max tokens seems too large for provider '{}': {}",
                        provider, max_tokens
                    )));
                }
            }

            // Validate top_k
            if let Some(top_k) = params.top_k {
                if top_k == 0 {
                    return Err(SageError::config(format!(
                        "Top-k must be greater than 0 for provider '{}'",
                        provider
                    )));
                }
            }

            // Validate max_retries
            if let Some(max_retries) = params.max_retries {
                if max_retries > 10 {
                    return Err(SageError::config(format!(
                        "Max retries seems too large for provider '{}': {}",
                        provider, max_retries
                    )));
                }
            }

            // Validate API key presence for cloud providers
            // Local providers like ollama don't require API keys
            let cloud_providers = [
                "openai",
                "anthropic",
                "google",
                "azure",
                "openrouter",
                "doubao",
                "glm",
                "zhipu",
            ];
            if cloud_providers.contains(&provider.as_str()) {
                if params.api_key.is_none() && params.get_api_key().is_none() {
                    return Err(SageError::config(format!(
                        "API key is required for provider '{}'. Set it in config or environment variables",
                        provider
                    )));
                }
            }

            // Validate base URL format
            if let Some(base_url) = &params.base_url {
                if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
                    return Err(SageError::config(format!(
                        "Base URL must start with http:// or https:// for provider '{}', got '{}'",
                        provider, base_url
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate limits and constraints
    fn validate_limits(config: &Config) -> SageResult<()> {
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

    /// Validate file paths
    fn validate_paths(config: &Config) -> SageResult<()> {
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

    /// Validate tool configuration
    fn validate_tools(config: &Config) -> SageResult<()> {
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
    fn test_validate_providers_success() {
        let config = create_test_config();
        assert!(ConfigValidator::validate_providers(&config).is_ok());
    }

    #[test]
    fn test_validate_providers_missing_default() {
        let mut config = create_test_config();
        config.default_provider = "nonexistent".to_string();

        let result = ConfigValidator::validate_providers(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Default provider"));
    }

    #[test]
    fn test_validate_providers_unknown_provider() {
        let mut config = create_test_config();
        config
            .model_providers
            .insert("unknown_provider".to_string(), ModelParameters::default());
        config.default_provider = "unknown_provider".to_string();

        let result = ConfigValidator::validate_providers(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown provider"));
    }

    #[test]
    fn test_validate_providers_custom_prefix_allowed() {
        let mut config = create_test_config();
        config
            .model_providers
            .insert("custom_my_llm".to_string(), ModelParameters::default());
        config.default_provider = "custom_my_llm".to_string();

        // Custom providers with custom_ prefix should be allowed
        assert!(ConfigValidator::validate_providers(&config).is_ok());
    }

    #[test]
    fn test_validate_models_empty_model_name() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.model = "".to_string();
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Model name cannot be empty")
        );
    }

    #[test]
    fn test_validate_models_invalid_temperature() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.temperature = Some(3.0); // Invalid: > 2.0
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Temperature"));
    }

    #[test]
    fn test_validate_models_negative_temperature() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.temperature = Some(-0.1); // Invalid: < 0.0
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_models_invalid_top_p() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.top_p = Some(1.5); // Invalid: > 1.0
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Top-p"));
    }

    #[test]
    fn test_validate_models_zero_max_tokens() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.max_tokens = Some(0);
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Max tokens must be greater than 0")
        );
    }

    #[test]
    fn test_validate_models_excessive_max_tokens() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.max_tokens = Some(2_000_000); // Too large
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Max tokens seems too large")
        );
    }

    #[test]
    fn test_validate_models_zero_top_k() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.top_k = Some(0);
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Top-k must be greater than 0")
        );
    }

    #[test]
    fn test_validate_models_excessive_retries() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.max_retries = Some(15); // Too many
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Max retries seems too large")
        );
    }

    #[test]
    fn test_validate_models_invalid_base_url() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.base_url = Some("invalid-url".to_string()); // No http(s)://
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Base URL must start with http")
        );
    }

    #[test]
    fn test_validate_models_missing_api_key_cloud_provider() {
        let mut config = create_test_config();
        let mut params = config.model_providers.get("anthropic").unwrap().clone();
        params.api_key = None; // Remove API key
        config
            .model_providers
            .insert("anthropic".to_string(), params);

        // Clear environment variables to ensure they don't interfere
        // SAFETY: This is test code running in single-threaded test context
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("GOOGLE_API_KEY");
        }

        let result = ConfigValidator::validate_models(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("API key is required")
        );
    }

    #[test]
    fn test_validate_limits_zero_max_steps() {
        let mut config = create_test_config();
        config.max_steps = Some(0);

        let result = ConfigValidator::validate_limits(&config);
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

        let result = ConfigValidator::validate_limits(&config);
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

        assert!(ConfigValidator::validate_limits(&config).is_ok());
    }

    #[test]
    fn test_validate_limits_zero_tool_execution_time() {
        let mut config = create_test_config();
        config.tools.max_execution_time = 0;

        let result = ConfigValidator::validate_limits(&config);
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

        let result = ConfigValidator::validate_limits(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Tool max execution time seems too large")
        );
    }

    #[test]
    fn test_validate_paths_nonexistent_working_directory() {
        let mut config = create_test_config();
        config.working_directory = Some(std::path::PathBuf::from(
            "/nonexistent/path/that/does/not/exist",
        ));

        let result = ConfigValidator::validate_paths(&config);
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

        let result = ConfigValidator::validate_paths(&config);

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

        let result = ConfigValidator::validate_paths(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Log file directory does not exist")
        );
    }

    #[test]
    fn test_validate_tools_unknown_tool() {
        let mut config = create_test_config();
        config.tools.enabled_tools.push("unknown_tool".to_string());

        let result = ConfigValidator::validate_tools(&config);
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
        assert!(ConfigValidator::validate_tools(&config).is_ok());
    }

    #[test]
    fn test_validate_tools_task_done_required() {
        let mut config = create_test_config();
        config.tools.enabled_tools = vec!["bash".to_string()]; // Missing task_done

        let result = ConfigValidator::validate_tools(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task_done"));
    }

    #[test]
    fn test_validate_logging_invalid_log_level() {
        let mut config = create_test_config();
        config.logging.level = "invalid".to_string();

        let result = ConfigValidator::validate_logging(&config);
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
            assert!(ConfigValidator::validate_logging(&config).is_ok());
        }
    }

    #[test]
    fn test_validate_logging_invalid_format() {
        let mut config = create_test_config();
        config.logging.format = "xml".to_string(); // Invalid format

        let result = ConfigValidator::validate_logging(&config);
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
            assert!(ConfigValidator::validate_logging(&config).is_ok());
        }
    }

    #[test]
    fn test_validate_logging_no_output_enabled() {
        let mut config = create_test_config();
        config.logging.log_to_console = false;
        config.logging.log_to_file = false;

        let result = ConfigValidator::validate_logging(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("At least one of log_to_console or log_to_file must be enabled")
        );
    }

    #[test]
    fn test_validate_lakeview_disabled() {
        let config = create_test_config();
        // Lakeview is disabled, should pass validation
        assert!(ConfigValidator::validate_lakeview(&config).is_ok());
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

        assert!(ConfigValidator::validate_lakeview(&config).is_ok());
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

        let result = ConfigValidator::validate_lakeview(&config);
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

        let result = ConfigValidator::validate_lakeview(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Lakeview model name cannot be empty")
        );
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
