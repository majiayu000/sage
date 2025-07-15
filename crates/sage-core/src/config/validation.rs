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
        if !config.model_providers.contains_key(&config.default_provider) {
            return Err(SageError::config(format!(
                "Default provider '{}' not found in model_providers",
                config.default_provider
            )));
        }

        // Validate provider names
        let valid_providers: HashSet<&str> = 
            ["openai", "anthropic", "google", "ollama"].iter().cloned().collect();

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
                if temp < 0.0 || temp > 2.0 {
                    return Err(SageError::config(format!(
                        "Temperature must be between 0.0 and 2.0 for provider '{}', got {}",
                        provider, temp
                    )));
                }
            }

            // Validate top_p
            if let Some(top_p) = params.top_p {
                if top_p < 0.0 || top_p > 1.0 {
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
            if ["openai", "anthropic", "google"].contains(&provider.as_str()) {
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
        // Validate max_steps
        if config.max_steps == 0 {
            return Err(SageError::config("Max steps must be greater than 0"));
        }
        if config.max_steps > 1000 {
            return Err(SageError::config(format!(
                "Max steps seems too large: {}. Consider using a smaller value",
                config.max_steps
            )));
        }

        // Validate tool execution time
        if config.tools.max_execution_time == 0 {
            return Err(SageError::config("Tool max execution time must be greater than 0"));
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
        ].iter().cloned().collect();

        for tool in &config.tools.enabled_tools {
            if !valid_tools.contains(tool.as_str()) && !tool.starts_with("custom_") {
                return Err(SageError::config(format!(
                    "Unknown tool '{}'. Valid tools are: {:?}",
                    tool, valid_tools
                )));
            }
        }

        // Ensure task_done tool is always enabled
        if !config.tools.enabled_tools.contains(&"task_done".to_string()) {
            return Err(SageError::config(
                "The 'task_done' tool must be enabled for proper agent operation"
            ));
        }

        Ok(())
    }

    /// Validate logging configuration
    pub fn validate_logging(config: &Config) -> SageResult<()> {
        // Validate log level
        let valid_levels: HashSet<&str> = 
            ["trace", "debug", "info", "warn", "error"].iter().cloned().collect();
        
        if !valid_levels.contains(config.logging.level.as_str()) {
            return Err(SageError::config(format!(
                "Invalid log level '{}'. Valid levels are: {:?}",
                config.logging.level, valid_levels
            )));
        }

        // Validate log format
        let valid_formats: HashSet<&str> = 
            ["json", "pretty", "compact"].iter().cloned().collect();
        
        if !valid_formats.contains(config.logging.format.as_str()) {
            return Err(SageError::config(format!(
                "Invalid log format '{}'. Valid formats are: {:?}",
                config.logging.format, valid_formats
            )));
        }

        // Ensure at least one output is enabled
        if !config.logging.log_to_console && !config.logging.log_to_file {
            return Err(SageError::config(
                "At least one of log_to_console or log_to_file must be enabled"
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
