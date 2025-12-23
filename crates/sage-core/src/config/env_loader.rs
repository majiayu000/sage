//! Environment variable-based configuration loading

use crate::config::model::{Config, ModelParameters};
use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

/// Load configuration from environment variables
///
/// Supports environment variables with SAGE_ prefix for general settings
/// and provider-specific variables (OPENAI_, ANTHROPIC_, etc.)
pub fn load_from_env() -> SageResult<Config> {
    let mut config = Config {
        default_provider: String::new(), // Don't set default here
        max_steps: None,                 // None = unlimited
        total_token_budget: None,
        model_providers: HashMap::new(),
        lakeview_config: None,
        enable_lakeview: false,
        working_directory: None,
        tools: crate::config::model::ToolConfig {
            enabled_tools: Vec::new(),
            tool_settings: HashMap::new(),
            max_execution_time: 0,
            allow_parallel_execution: false,
        },
        logging: crate::config::model::LoggingConfig::default(),
        trajectory: crate::config::model::TrajectoryConfig::default(),
        mcp: crate::config::model::McpConfig::default(),
    };

    // Load provider settings
    if let Ok(provider) = env::var("SAGE_DEFAULT_PROVIDER") {
        config.default_provider = provider;
    }

    if let Ok(max_steps_str) = env::var("SAGE_MAX_STEPS") {
        let max_steps: u32 = max_steps_str
            .parse()
            .map_err(|_| SageError::config("Invalid SAGE_MAX_STEPS value"))?;
        config.max_steps = Some(max_steps);
    }

    // Load model parameters for different providers
    load_provider_from_env(&mut config, "openai", "OPENAI")?;
    load_provider_from_env(&mut config, "anthropic", "ANTHROPIC")?;
    load_provider_from_env(&mut config, "google", "GOOGLE")?;
    load_provider_from_env(&mut config, "ollama", "OLLAMA")?;

    // Load working directory
    if let Ok(working_dir) = env::var("SAGE_WORKING_DIR") {
        config.working_directory = Some(PathBuf::from(working_dir));
    }

    // Load Lakeview settings
    if let Ok(enable_lakeview) = env::var("SAGE_ENABLE_LAKEVIEW") {
        config.enable_lakeview = enable_lakeview.parse().unwrap_or(false);
    }

    Ok(config)
}

/// Load provider configuration from environment variables
fn load_provider_from_env(
    config: &mut Config,
    provider: &str,
    env_prefix: &str,
) -> SageResult<()> {
    let mut params = ModelParameters::default();
    let mut has_config = false;

    // API Key
    if let Ok(api_key) = env::var(format!("{}_API_KEY", env_prefix)) {
        params.api_key = Some(api_key);
        has_config = true;
    }

    // Model
    if let Ok(model) = env::var(format!("{}_MODEL", env_prefix)) {
        params.model = model;
        has_config = true;
    }

    // Base URL
    if let Ok(base_url) = env::var(format!("{}_BASE_URL", env_prefix)) {
        params.base_url = Some(base_url);
        has_config = true;
    }

    // Temperature
    if let Ok(temp) = env::var(format!("{}_TEMPERATURE", env_prefix)) {
        params.temperature = Some(temp.parse().map_err(|_| {
            SageError::config_with_context(
                format!("Invalid {}_TEMPERATURE value", env_prefix),
                format!(
                    "Parsing temperature value '{}' for provider '{}'",
                    temp, provider
                ),
            )
        })?);
        has_config = true;
    }

    // Max tokens
    if let Ok(max_tokens) = env::var(format!("{}_MAX_TOKENS", env_prefix)) {
        params.max_tokens = Some(max_tokens.parse().map_err(|_| {
            SageError::config_with_context(
                format!("Invalid {}_MAX_TOKENS value", env_prefix),
                format!(
                    "Parsing max_tokens value '{}' for provider '{}'",
                    max_tokens, provider
                ),
            )
        })?);
        has_config = true;
    }

    if has_config {
        config.model_providers.insert(provider.to_string(), params);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_env_basic() {
        unsafe {
            std::env::set_var("SAGE_DEFAULT_PROVIDER", "google");
            std::env::set_var("SAGE_MAX_STEPS", "75");
        }

        let config = load_from_env().unwrap();
        assert_eq!(config.default_provider, "google");
        assert_eq!(config.max_steps, Some(75));

        unsafe {
            std::env::remove_var("SAGE_DEFAULT_PROVIDER");
            std::env::remove_var("SAGE_MAX_STEPS");
        }
    }

    #[test]
    fn test_load_from_env_with_provider() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "env_test_key");
            std::env::set_var("OPENAI_MODEL", "gpt-4-turbo");
        }

        let config = load_from_env().unwrap();

        if let Some(openai_params) = config.model_providers.get("openai") {
            assert_eq!(openai_params.model, "gpt-4-turbo");
            assert_eq!(openai_params.api_key, Some("env_test_key".to_string()));
        } else {
            panic!("OpenAI provider should be loaded");
        }

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("OPENAI_MODEL");
        }
    }

    #[test]
    fn test_load_provider_temperature_and_tokens() {
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "test_key");
            std::env::set_var("ANTHROPIC_MODEL", "claude-3");
            std::env::set_var("ANTHROPIC_TEMPERATURE", "0.9");
            std::env::set_var("ANTHROPIC_MAX_TOKENS", "8192");
        }

        let config = load_from_env().unwrap();

        if let Some(params) = config.model_providers.get("anthropic") {
            assert_eq!(params.temperature, Some(0.9));
            assert_eq!(params.max_tokens, Some(8192));
        }

        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("ANTHROPIC_MODEL");
            std::env::remove_var("ANTHROPIC_TEMPERATURE");
            std::env::remove_var("ANTHROPIC_MAX_TOKENS");
        }
    }

    #[test]
    fn test_load_provider_invalid_temperature() {
        unsafe {
            std::env::set_var("GOOGLE_API_KEY", "test_key");
            std::env::set_var("GOOGLE_TEMPERATURE", "invalid");
        }

        let result = load_from_env();
        assert!(result.is_err());

        unsafe {
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_TEMPERATURE");
        }
    }

    #[test]
    fn test_load_provider_base_url() {
        unsafe {
            std::env::set_var("OLLAMA_BASE_URL", "http://localhost:11434");
            std::env::set_var("OLLAMA_MODEL", "llama2");
        }

        let config = load_from_env().unwrap();

        if let Some(params) = config.model_providers.get("ollama") {
            assert_eq!(params.base_url, Some("http://localhost:11434".to_string()));
        }

        unsafe {
            std::env::remove_var("OLLAMA_BASE_URL");
            std::env::remove_var("OLLAMA_MODEL");
        }
    }

    #[test]
    fn test_load_lakeview_enabled() {
        unsafe {
            std::env::set_var("SAGE_ENABLE_LAKEVIEW", "true");
        }

        let config = load_from_env().unwrap();
        assert!(config.enable_lakeview);

        unsafe {
            std::env::remove_var("SAGE_ENABLE_LAKEVIEW");
        }
    }

    #[test]
    fn test_load_multiple_providers() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "openai_key");
            std::env::set_var("OPENAI_MODEL", "gpt-4");
            std::env::set_var("ANTHROPIC_API_KEY", "anthropic_key");
            std::env::set_var("ANTHROPIC_MODEL", "claude-3");
            std::env::set_var("GOOGLE_API_KEY", "google_key");
            std::env::set_var("GOOGLE_MODEL", "gemini-pro");
        }

        let config = load_from_env().unwrap();

        assert!(config.model_providers.contains_key("openai"));
        assert!(config.model_providers.contains_key("anthropic"));
        assert!(config.model_providers.contains_key("google"));

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("OPENAI_MODEL");
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("ANTHROPIC_MODEL");
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_MODEL");
        }
    }
}
