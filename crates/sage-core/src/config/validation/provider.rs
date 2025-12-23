//! Provider configuration validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use std::collections::HashSet;

/// Validate provider configuration
pub fn validate_providers(config: &Config) -> SageResult<()> {
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
    // All providers from LlmProvider enum in llm/providers.rs
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
        assert!(validate_providers(&config).is_ok());
    }

    #[test]
    fn test_validate_providers_missing_default() {
        let mut config = create_test_config();
        config.default_provider = "nonexistent".to_string();

        let result = validate_providers(&config);
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

        let result = validate_providers(&config);
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
        assert!(validate_providers(&config).is_ok());
    }
}
