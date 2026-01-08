//! Model parameters for LLM providers

use crate::config::provider::{ApiKeyInfo, ApiKeySource};
use crate::error::{SageError, SageResult};
use serde::{Deserialize, Serialize};

/// Model parameters for LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Model name/ID
    pub model: String,
    /// API key for the provider
    pub api_key: Option<String>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Top-k sampling (for supported models)
    pub top_k: Option<u32>,
    /// Whether to enable parallel tool calls
    pub parallel_tool_calls: Option<bool>,
    /// Maximum retries for failed requests
    pub max_retries: Option<u32>,
    /// Base URL for the API
    pub base_url: Option<String>,
    /// API version
    pub api_version: Option<String>,
    /// Stop sequences
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            api_key: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(3),
            base_url: None,
            api_version: None,
            stop_sequences: None,
        }
    }
}

impl ModelParameters {
    /// Get API key from environment or config
    pub fn get_api_key(&self) -> Option<String> {
        self.get_api_key_for_provider("default").key
    }

    /// Get detailed API key information for a specific provider
    ///
    /// Priority order:
    /// 1. SAGE_<PROVIDER>_API_KEY environment variable
    /// 2. Standard provider environment variable (e.g., ANTHROPIC_API_KEY)
    /// 3. Configuration file
    pub fn get_api_key_info_for_provider(&self, provider: &str) -> ApiKeyInfo {
        let provider_upper = provider.to_uppercase();

        // 1. Try SAGE-prefixed env var first (highest priority)
        let sage_env_var = format!("SAGE_{}_API_KEY", provider_upper);
        if let Ok(key) = std::env::var(&sage_env_var) {
            if !key.is_empty() {
                return ApiKeyInfo {
                    key: Some(key),
                    source: ApiKeySource::SageEnvVar,
                    env_var_name: Some(sage_env_var),
                };
            }
        }

        // 2. Try standard environment variables
        let standard_env_vars = get_standard_env_vars_for_provider(provider);
        for env_var in standard_env_vars {
            if let Ok(key) = std::env::var(&env_var) {
                if !key.is_empty() {
                    return ApiKeyInfo {
                        key: Some(key),
                        source: ApiKeySource::StandardEnvVar,
                        env_var_name: Some(env_var),
                    };
                }
            }
        }

        // 3. Fall back to config file
        if let Some(api_key) = &self.api_key {
            if !api_key.is_empty() {
                return ApiKeyInfo {
                    key: Some(api_key.clone()),
                    source: ApiKeySource::ConfigFile,
                    env_var_name: None,
                };
            }
        }

        // No API key found
        ApiKeyInfo {
            key: None,
            source: ApiKeySource::NotFound,
            env_var_name: None,
        }
    }

    /// Get API key info (alias for get_api_key_info_for_provider with default)
    pub fn get_api_key_for_provider(&self, provider: &str) -> ApiKeyInfo {
        self.get_api_key_info_for_provider(provider)
    }

    /// Validate the API key format for a specific provider
    pub fn validate_api_key_format_for_provider(&self, provider: &str) -> Result<(), String> {
        // Ollama doesn't need an API key
        if provider == "ollama" {
            return Ok(());
        }

        let key_info = self.get_api_key_info_for_provider(provider);
        let key = match &key_info.key {
            Some(k) => k,
            None => {
                return Err(format!(
                    "API key required for '{}'. Set via {} or config file",
                    provider,
                    get_standard_env_vars_for_provider(provider)
                        .first()
                        .cloned()
                        .unwrap_or_default()
                ));
            }
        };

        // Provider-specific validation
        match provider {
            "anthropic" => {
                if !key.starts_with("sk-ant-") {
                    return Err("Anthropic API key should start with 'sk-ant-'".to_string());
                }
            }
            "openai" => {
                if !key.starts_with("sk-") {
                    return Err("OpenAI API key should start with 'sk-'".to_string());
                }
            }
            "google" => {
                if key.len() < 20 {
                    return Err("Google API key appears too short".to_string());
                }
            }
            "glm" => {
                if key.len() < 10 {
                    return Err("GLM API key appears too short".to_string());
                }
            }
            _ => {
                if key.is_empty() || key.contains("your-") || key.contains("xxx") {
                    return Err("API key appears to be a placeholder".to_string());
                }
            }
        }

        Ok(())
    }

    /// Get base URL for the provider
    pub fn get_base_url(&self) -> String {
        if let Some(base_url) = &self.base_url {
            base_url.clone()
        } else {
            // Default base URLs for different providers
            // Note: This is a fallback, provider should be determined by context
            "https://api.openai.com/v1".to_string()
        }
    }

    /// Get base URL for a specific provider
    pub fn get_base_url_for_provider(&self, provider: &str) -> String {
        if let Some(base_url) = &self.base_url {
            base_url.clone()
        } else {
            match provider {
                "openai" => "https://api.openai.com/v1".to_string(),
                "anthropic" => "https://api.anthropic.com".to_string(),
                "google" => "https://generativelanguage.googleapis.com".to_string(),
                "ollama" => "http://localhost:11434".to_string(),
                _ => "http://localhost:8000".to_string(),
            }
        }
    }

    /// Convert to LLM model parameters
    pub fn to_llm_parameters(&self) -> crate::llm::provider_types::ModelParameters {
        crate::llm::provider_types::ModelParameters {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stop: self.stop_sequences.clone(),
            parallel_tool_calls: self.parallel_tool_calls,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            enable_prompt_caching: None,
        }
    }

    /// Validate the model parameters
    pub fn validate(&self) -> SageResult<()> {
        if self.model.is_empty() {
            return Err(SageError::config("Model name cannot be empty"));
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(SageError::config("Temperature must be between 0.0 and 2.0"));
            }
        }

        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(SageError::config("Top-p must be between 0.0 and 1.0"));
            }
        }

        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 {
                return Err(SageError::config("Max tokens must be greater than 0"));
            }
        }

        Ok(())
    }

    /// Deep merge with another ModelParameters
    ///
    /// Fields from `other` override self only if they have a value (Some).
    /// This allows partial config overrides where user only specifies fields
    /// they want to change, while keeping defaults for others.
    ///
    /// # Example
    /// ```
    /// use sage_core::config::model::ModelParameters;
    ///
    /// let mut base = ModelParameters::default();
    /// let override_params = ModelParameters {
    ///     max_tokens: Some(8192),  // Override this
    ///     temperature: None,       // Keep base value
    ///     ..Default::default()
    /// };
    ///
    /// base.merge(override_params);
    /// // base.max_tokens is now Some(8192)
    /// // base.temperature is still Some(0.7) (default)
    /// ```
    pub fn merge(&mut self, other: Self) {
        // Model name: override if other has non-default value
        // We treat empty string as "not set" to allow keeping base model
        if !other.model.is_empty() && other.model != "gpt-4" {
            self.model = other.model;
        }

        // API key: override if other has a value
        if other.api_key.is_some() {
            self.api_key = other.api_key;
        }

        // Optional fields: override if other has Some value
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        if other.temperature.is_some() {
            self.temperature = other.temperature;
        }
        if other.top_p.is_some() {
            self.top_p = other.top_p;
        }
        if other.top_k.is_some() {
            self.top_k = other.top_k;
        }
        if other.parallel_tool_calls.is_some() {
            self.parallel_tool_calls = other.parallel_tool_calls;
        }
        if other.max_retries.is_some() {
            self.max_retries = other.max_retries;
        }
        if other.base_url.is_some() {
            self.base_url = other.base_url;
        }
        if other.api_version.is_some() {
            self.api_version = other.api_version;
        }
        if other.stop_sequences.is_some() {
            self.stop_sequences = other.stop_sequences;
        }
    }
}

/// Get standard environment variable names for a provider
fn get_standard_env_vars_for_provider(provider: &str) -> Vec<String> {
    match provider {
        "openai" => vec!["OPENAI_API_KEY".to_string()],
        "anthropic" => vec![
            "ANTHROPIC_API_KEY".to_string(),
            "CLAUDE_API_KEY".to_string(),
        ],
        "google" => vec!["GOOGLE_API_KEY".to_string(), "GEMINI_API_KEY".to_string()],
        "azure" => vec![
            "AZURE_OPENAI_API_KEY".to_string(),
            "AZURE_API_KEY".to_string(),
        ],
        "openrouter" => vec!["OPENROUTER_API_KEY".to_string()],
        "doubao" => vec!["DOUBAO_API_KEY".to_string(), "ARK_API_KEY".to_string()],
        "glm" | "zhipu" => vec!["GLM_API_KEY".to_string(), "ZHIPU_API_KEY".to_string()],
        _ => {
            // For custom or default providers, try <PROVIDER>_API_KEY
            vec![format!("{}_API_KEY", provider.to_uppercase())]
        }
    }
}

/// Format API key status for display
pub fn format_api_key_status_for_provider(provider: &str, info: &ApiKeyInfo) -> String {
    match &info.source {
        ApiKeySource::ConfigFile => {
            format!(
                "✓ {} API key (from config): {}",
                provider,
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::SageEnvVar | ApiKeySource::StandardEnvVar => {
            format!(
                "✓ {} API key (from {}): {}",
                provider,
                info.env_var_name.as_deref().unwrap_or("env"),
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::NotFound => {
            let env_hints = get_standard_env_vars_for_provider(provider);
            format!(
                "✗ {} API key missing. Set {} or add to config",
                provider,
                env_hints.first().cloned().unwrap_or_default()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_parameters_default() {
        let params = ModelParameters::default();
        assert_eq!(params.model, "gpt-4");
        assert_eq!(params.max_tokens, Some(4096));
        assert_eq!(params.temperature, Some(0.7));
        assert_eq!(params.top_p, Some(1.0));
        assert_eq!(params.parallel_tool_calls, Some(true));
        assert_eq!(params.max_retries, Some(3));
    }

    #[test]
    fn test_model_parameters_get_api_key_from_config() {
        let params = ModelParameters {
            api_key: Some("test_key".to_string()),
            ..Default::default()
        };
        assert_eq!(params.get_api_key(), Some("test_key".to_string()));
    }

    #[test]
    fn test_model_parameters_get_api_key_from_env() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "env_key");
        }

        let params = ModelParameters {
            api_key: None,
            ..Default::default()
        };
        // Use provider-specific method
        let key_info = params.get_api_key_info_for_provider("openai");
        assert_eq!(key_info.key, Some("env_key".to_string()));
        assert_eq!(key_info.source, ApiKeySource::StandardEnvVar);

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn test_model_parameters_get_base_url() {
        let params = ModelParameters {
            base_url: Some("https://custom.api".to_string()),
            ..Default::default()
        };
        assert_eq!(params.get_base_url(), "https://custom.api");
    }

    #[test]
    fn test_model_parameters_get_base_url_default() {
        let params = ModelParameters {
            base_url: None,
            ..Default::default()
        };
        assert_eq!(params.get_base_url(), "https://api.openai.com/v1");
    }

    #[test]
    fn test_model_parameters_get_base_url_for_provider() {
        let params = ModelParameters::default();

        assert_eq!(
            params.get_base_url_for_provider("openai"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            params.get_base_url_for_provider("anthropic"),
            "https://api.anthropic.com"
        );
        assert_eq!(
            params.get_base_url_for_provider("google"),
            "https://generativelanguage.googleapis.com"
        );
        assert_eq!(
            params.get_base_url_for_provider("ollama"),
            "http://localhost:11434"
        );
        assert_eq!(
            params.get_base_url_for_provider("unknown"),
            "http://localhost:8000"
        );
    }

    #[test]
    fn test_model_parameters_validate_success() {
        let params = ModelParameters {
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(4096),
            ..Default::default()
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_model_parameters_validate_empty_model() {
        let params = ModelParameters {
            model: "".to_string(),
            ..Default::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_model_parameters_validate_invalid_temperature() {
        let params = ModelParameters {
            model: "gpt-4".to_string(),
            temperature: Some(3.0), // > 2.0
            ..Default::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_model_parameters_validate_invalid_top_p() {
        let params = ModelParameters {
            model: "gpt-4".to_string(),
            top_p: Some(1.5), // > 1.0
            ..Default::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_model_parameters_validate_zero_max_tokens() {
        let params = ModelParameters {
            model: "gpt-4".to_string(),
            max_tokens: Some(0),
            ..Default::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_model_parameters_to_llm_parameters() {
        let params = ModelParameters {
            model: "gpt-4".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            stop_sequences: Some(vec!["STOP".to_string()]),
            parallel_tool_calls: Some(true),
            ..Default::default()
        };

        let llm_params = params.to_llm_parameters();
        assert_eq!(llm_params.model, "gpt-4");
        assert_eq!(llm_params.max_tokens, Some(4096));
        assert_eq!(llm_params.temperature, Some(0.7));
        assert_eq!(llm_params.top_p, Some(0.9));
        assert_eq!(llm_params.top_k, Some(40));
        assert_eq!(llm_params.stop, Some(vec!["STOP".to_string()]));
        assert_eq!(llm_params.parallel_tool_calls, Some(true));
    }

    #[test]
    fn test_model_parameters_debug() {
        let params = ModelParameters::default();
        let debug_string = format!("{:?}", params);
        assert!(debug_string.contains("ModelParameters"));
    }

    #[test]
    fn test_model_parameters_clone() {
        let params = ModelParameters::default();
        let cloned = params.clone();
        assert_eq!(params.model, cloned.model);
    }

    #[test]
    fn test_model_parameters_merge_partial_override() {
        let mut base = ModelParameters {
            model: "claude-3-sonnet".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(0.9),
            ..Default::default()
        };

        let override_params = ModelParameters {
            model: "".to_string(),  // Empty = don't override
            max_tokens: Some(8192), // Override this
            temperature: None,      // None = keep base
            top_p: None,            // None = keep base
            ..Default::default()
        };

        base.merge(override_params);

        // model should be unchanged (empty string doesn't override)
        assert_eq!(base.model, "claude-3-sonnet");
        // max_tokens should be overridden
        assert_eq!(base.max_tokens, Some(8192));
        // temperature should be preserved
        assert_eq!(base.temperature, Some(0.7));
        // top_p should be preserved
        assert_eq!(base.top_p, Some(0.9));
    }

    #[test]
    fn test_model_parameters_merge_api_key() {
        let mut base = ModelParameters {
            api_key: Some("base_key".to_string()),
            ..Default::default()
        };

        // Override with None should preserve base
        let no_key = ModelParameters {
            api_key: None,
            ..Default::default()
        };
        base.merge(no_key);
        assert_eq!(base.api_key, Some("base_key".to_string()));

        // Override with Some should replace
        let new_key = ModelParameters {
            api_key: Some("new_key".to_string()),
            ..Default::default()
        };
        base.merge(new_key);
        assert_eq!(base.api_key, Some("new_key".to_string()));
    }

    #[test]
    fn test_model_parameters_merge_model_name() {
        let mut base = ModelParameters {
            model: "claude-3-sonnet".to_string(),
            ..Default::default()
        };

        // Override with custom model
        let custom = ModelParameters {
            model: "claude-3-opus".to_string(),
            ..Default::default()
        };
        base.merge(custom);
        assert_eq!(base.model, "claude-3-opus");

        // Empty string should not override
        let empty = ModelParameters {
            model: "".to_string(),
            ..Default::default()
        };
        base.merge(empty);
        assert_eq!(base.model, "claude-3-opus"); // Unchanged
    }

    #[test]
    fn test_model_parameters_merge_all_fields() {
        let mut base = ModelParameters::default();

        let override_all = ModelParameters {
            model: "custom-model".to_string(),
            api_key: Some("key".to_string()),
            max_tokens: Some(16384),
            temperature: Some(0.5),
            top_p: Some(0.8),
            top_k: Some(50),
            parallel_tool_calls: Some(false),
            max_retries: Some(5),
            base_url: Some("https://custom.api".to_string()),
            api_version: Some("2024-01".to_string()),
            stop_sequences: Some(vec!["END".to_string()]),
        };

        base.merge(override_all);

        assert_eq!(base.model, "custom-model");
        assert_eq!(base.api_key, Some("key".to_string()));
        assert_eq!(base.max_tokens, Some(16384));
        assert_eq!(base.temperature, Some(0.5));
        assert_eq!(base.top_p, Some(0.8));
        assert_eq!(base.top_k, Some(50));
        assert_eq!(base.parallel_tool_calls, Some(false));
        assert_eq!(base.max_retries, Some(5));
        assert_eq!(base.base_url, Some("https://custom.api".to_string()));
        assert_eq!(base.api_version, Some("2024-01".to_string()));
        assert_eq!(base.stop_sequences, Some(vec!["END".to_string()]));
    }
}
