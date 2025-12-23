//! Model parameters for LLM providers

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
        self.api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
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
        assert_eq!(params.get_api_key(), Some("env_key".to_string()));

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
}
