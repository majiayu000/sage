//! Command line arguments-based configuration loading

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::path::PathBuf;

/// Load configuration from command line arguments
///
/// Supports args for provider, model, api_key, model_base_url, max_steps, and working_dir.
pub fn load_from_args(args: &HashMap<String, String>) -> SageResult<Config> {
    let mut config = Config {
        default_provider: String::new(), // Don't set default here
        max_steps: None,                 // None = unlimited
        total_token_budget: None,
        model_providers: HashMap::new(),
        lakeview_config: None,
        enable_lakeview: false,
        working_directory: None,
        tools: crate::config::model::ToolConfig {
            tool_settings: HashMap::new(),
            max_execution_time: 0,
            allow_parallel_execution: false,
        },
        logging: crate::config::model::LoggingConfig::default(),
        trajectory: crate::config::model::TrajectoryConfig::default(),
        mcp: crate::config::model::McpConfig::default(),
    };

    if let Some(provider) = args.get("provider") {
        config.default_provider = provider.clone();
    }

    if let Some(model) = args.get("model") {
        // Update the model for the current provider
        let provider = config.default_provider.clone();
        let mut params = config
            .model_providers
            .get(&provider)
            .cloned()
            .unwrap_or_default();
        params.model = model.clone();
        config.model_providers.insert(provider, params);
    }

    if let Some(api_key) = args.get("api_key") {
        let provider = config.default_provider.clone();
        let mut params = config
            .model_providers
            .get(&provider)
            .cloned()
            .unwrap_or_default();
        params.api_key = Some(api_key.clone());
        config.model_providers.insert(provider, params);
    }

    if let Some(base_url) = args.get("model_base_url") {
        let provider = config.default_provider.clone();
        let mut params = config
            .model_providers
            .get(&provider)
            .cloned()
            .unwrap_or_default();
        params.base_url = Some(base_url.clone());
        config.model_providers.insert(provider, params);
    }

    if let Some(max_steps_str) = args.get("max_steps") {
        let max_steps: u32 = max_steps_str.parse().map_err(|_| {
            SageError::config_with_context(
                "Invalid max_steps value",
                format!(
                    "Parsing max_steps value '{}' from command line arguments",
                    max_steps_str
                ),
            )
        })?;
        config.max_steps = Some(max_steps);
    }

    if let Some(working_dir) = args.get("working_dir") {
        config.working_directory = Some(PathBuf::from(working_dir));
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_args_basic() {
        let args = HashMap::from([
            ("provider".to_string(), "anthropic".to_string()),
            ("max_steps".to_string(), "25".to_string()),
        ]);

        let config = load_from_args(&args).unwrap();
        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, Some(25));
    }

    #[test]
    fn test_load_from_args_with_model_and_api_key() {
        let args = HashMap::from([
            ("provider".to_string(), "openai".to_string()),
            ("model".to_string(), "gpt-4-turbo".to_string()),
            ("api_key".to_string(), "test_api_key_from_args".to_string()),
        ]);

        let config = load_from_args(&args).unwrap();
        assert_eq!(config.default_provider, "openai");

        if let Some(params) = config.model_providers.get("openai") {
            assert_eq!(params.model, "gpt-4-turbo");
            assert_eq!(params.api_key, Some("test_api_key_from_args".to_string()));
        } else {
            panic!("OpenAI provider should be configured from args");
        }
    }

    #[test]
    fn test_load_from_args_with_base_url() {
        let args = HashMap::from([
            ("provider".to_string(), "ollama".to_string()),
            (
                "model_base_url".to_string(),
                "http://custom-host:8080".to_string(),
            ),
        ]);

        let config = load_from_args(&args).unwrap();

        if let Some(params) = config.model_providers.get("ollama") {
            assert_eq!(params.base_url, Some("http://custom-host:8080".to_string()));
        }
    }

    #[test]
    fn test_load_from_args_invalid_max_steps() {
        let args = HashMap::from([("max_steps".to_string(), "invalid".to_string())]);

        let result = load_from_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_args_with_working_dir() {
        let args = HashMap::from([("working_dir".to_string(), "/tmp/test".to_string())]);

        let config = load_from_args(&args).unwrap();
        assert_eq!(config.working_directory, Some(PathBuf::from("/tmp/test")));
    }
}
