//! Model parameter validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};

/// Validate model configurations
pub fn validate_models(config: &Config) -> SageResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{
        LoggingConfig, McpConfig, ModelParameters, ToolConfig, TrajectoryConfig,
    };
    use std::collections::HashMap;

    fn create_minimal_config(params: ModelParameters) -> Config {
        let mut model_providers = HashMap::new();
        model_providers.insert("anthropic".to_string(), params);

        Config {
            default_provider: "anthropic".to_string(),
            max_steps: Some(50),
            total_token_budget: Some(100000),
            model_providers,
            lakeview_config: None,
            enable_lakeview: false,
            working_directory: Some(std::env::temp_dir()),
            tools: ToolConfig {
                enabled_tools: vec!["task_done".to_string()],
                tool_settings: HashMap::new(),
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
    fn test_validate_models_empty_model_name() {
        let params = ModelParameters {
            model: "".to_string(),
            api_key: Some("key".to_string()),
            ..Default::default()
        };
        assert!(validate_models(&create_minimal_config(params)).is_err());
    }

    #[test]
    fn test_validate_models_invalid_temperature() {
        let params = ModelParameters {
            model: "m".to_string(),
            api_key: Some("k".to_string()),
            temperature: Some(3.0),
            ..Default::default()
        };
        let result = validate_models(&create_minimal_config(params));
        assert!(result.is_err() && result.unwrap_err().to_string().contains("Temperature"));
    }

    #[test]
    fn test_validate_models_zero_max_tokens() {
        let params = ModelParameters {
            model: "m".to_string(),
            api_key: Some("k".to_string()),
            max_tokens: Some(0),
            ..Default::default()
        };
        let result = validate_models(&create_minimal_config(params));
        assert!(
            result.is_err()
                && result
                    .unwrap_err()
                    .to_string()
                    .contains("Max tokens must be greater than 0")
        );
    }

    #[test]
    fn test_validate_models_excessive_max_tokens() {
        let params = ModelParameters {
            model: "m".to_string(),
            api_key: Some("k".to_string()),
            max_tokens: Some(2_000_000),
            ..Default::default()
        };
        let result = validate_models(&create_minimal_config(params));
        assert!(
            result.is_err()
                && result
                    .unwrap_err()
                    .to_string()
                    .contains("Max tokens seems too large")
        );
    }

    #[test]
    fn test_validate_models_zero_top_k() {
        let params = ModelParameters {
            model: "m".to_string(),
            api_key: Some("k".to_string()),
            top_k: Some(0),
            ..Default::default()
        };
        let result = validate_models(&create_minimal_config(params));
        assert!(
            result.is_err()
                && result
                    .unwrap_err()
                    .to_string()
                    .contains("Top-k must be greater than 0")
        );
    }

    #[test]
    fn test_validate_models_invalid_base_url() {
        let params = ModelParameters {
            model: "m".to_string(),
            api_key: Some("k".to_string()),
            base_url: Some("invalid".to_string()),
            ..Default::default()
        };
        let result = validate_models(&create_minimal_config(params));
        assert!(
            result.is_err()
                && result
                    .unwrap_err()
                    .to_string()
                    .contains("Base URL must start with http")
        );
    }

    #[test]
    fn test_validate_models_missing_api_key_cloud_provider() {
        let params = ModelParameters {
            model: "m".to_string(),
            api_key: None,
            ..Default::default()
        };
        // SAFETY: Test code in single-threaded context
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("GOOGLE_API_KEY");
        }
        let result = validate_models(&create_minimal_config(params));
        assert!(
            result.is_err()
                && result
                    .unwrap_err()
                    .to_string()
                    .contains("API key is required")
        );
    }
}
