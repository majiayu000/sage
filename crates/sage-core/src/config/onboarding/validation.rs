//! Validation types and logic for onboarding
//!
//! This module provides types and functions for API key validation.

/// Result of attempting to validate an API key
#[derive(Debug, Clone)]
pub struct ApiKeyValidationResult {
    /// Whether the key is valid
    pub valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Model information if valid
    pub model_info: Option<String>,
}

impl ApiKeyValidationResult {
    pub fn success(model_info: impl Into<String>) -> Self {
        Self {
            valid: true,
            error: None,
            model_info: Some(model_info.into()),
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            error: Some(error.into()),
            model_info: None,
        }
    }
}

/// Validate an API key format for a specific provider
pub async fn validate_api_key_format(provider: &str, api_key: &str) -> ApiKeyValidationResult {
    match provider {
        "anthropic" => {
            if !api_key.starts_with("sk-ant-") && !api_key.starts_with("sk-") {
                return ApiKeyValidationResult::failure(
                    "Anthropic API keys typically start with 'sk-ant-' or 'sk-'",
                );
            }
        }
        "openai" => {
            if !api_key.starts_with("sk-") {
                return ApiKeyValidationResult::failure(
                    "OpenAI API keys typically start with 'sk-'",
                );
            }
        }
        "google" => {
            if api_key.len() < 30 {
                return ApiKeyValidationResult::failure("Google API keys are typically longer");
            }
        }
        "glm" => {
            if api_key.len() < 20 {
                return ApiKeyValidationResult::failure(
                    "智谱AI API keys are typically longer (20+ characters)",
                );
            }
        }
        "ollama" => {
            return ApiKeyValidationResult::success("Ollama configured (local)");
        }
        _ => {}
    }

    ApiKeyValidationResult::success(format!("{} API key format valid", provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_success() {
        let result = ApiKeyValidationResult::success("Model: claude-3");
        assert!(result.valid);
        assert!(result.error.is_none());
        assert_eq!(result.model_info, Some("Model: claude-3".to_string()));
    }

    #[test]
    fn test_validation_result_failure() {
        let result = ApiKeyValidationResult::failure("Invalid key");
        assert!(!result.valid);
        assert_eq!(result.error, Some("Invalid key".to_string()));
        assert!(result.model_info.is_none());
    }

    #[tokio::test]
    async fn test_validate_anthropic_key() {
        let result = validate_api_key_format("anthropic", "sk-ant-test").await;
        assert!(result.valid);

        let result = validate_api_key_format("anthropic", "invalid").await;
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_validate_openai_key() {
        let result = validate_api_key_format("openai", "sk-test").await;
        assert!(result.valid);

        let result = validate_api_key_format("openai", "invalid").await;
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_validate_ollama() {
        let result = validate_api_key_format("ollama", "anything").await;
        assert!(result.valid);
    }
}
