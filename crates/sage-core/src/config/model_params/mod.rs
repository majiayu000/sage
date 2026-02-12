//! Model parameters for LLM providers

use crate::config::api_key_helpers::get_standard_env_vars_for_provider;
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
                // Handle ${VAR_NAME} placeholder format
                if api_key.starts_with("${") && api_key.ends_with("}") {
                    let var_name = &api_key[2..api_key.len() - 1];
                    if let Ok(key) = std::env::var(var_name) {
                        if !key.is_empty() {
                            return ApiKeyInfo {
                                key: Some(key),
                                source: ApiKeySource::StandardEnvVar,
                                env_var_name: Some(var_name.to_string()),
                            };
                        }
                    }
                    // Placeholder not resolved, treat as not found
                } else {
                    return ApiKeyInfo {
                        key: Some(api_key.clone()),
                        source: ApiKeySource::ConfigFile,
                        env_var_name: None,
                    };
                }
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
    pub fn to_llm_parameters(&self) -> crate::llm::provider_types::LlmRequestParams {
        crate::llm::provider_types::LlmRequestParams {
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
    pub fn merge(&mut self, other: Self) {
        if !other.model.is_empty() && other.model != "gpt-4" {
            self.model = other.model;
        }
        if other.api_key.is_some() {
            self.api_key = other.api_key;
        }
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

#[cfg(test)]
mod tests;
