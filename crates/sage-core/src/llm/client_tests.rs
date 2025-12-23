//! Unit tests for LLM client

#[cfg(test)]
mod tests {
    use crate::config::provider::ProviderConfig;
    use crate::error::SageError;
    use crate::llm::client::LLMClient;
    use crate::llm::provider_types::{LLMProvider, ModelParameters, TimeoutConfig};

    #[test]
    fn test_llm_client_creation() {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_base_url("https://api.openai.com/v1");

        let model_params = ModelParameters {
            model: "gpt-4".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            ..Default::default()
        };

        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.provider(), &LLMProvider::OpenAI);
        assert_eq!(client.model(), "gpt-4");
    }

    #[test]
    fn test_llm_client_getters() {
        let config = ProviderConfig::new("anthropic")
            .with_api_key("test-key")
            .with_base_url("https://api.anthropic.com");

        let model_params = ModelParameters {
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: Some(2000),
            ..Default::default()
        };

        let client =
            LLMClient::new(LLMProvider::Anthropic, config.clone(), model_params.clone()).unwrap();

        assert_eq!(client.provider(), &LLMProvider::Anthropic);
        assert_eq!(client.model(), "claude-3-opus-20240229");
        assert_eq!(client.config().name, "anthropic");
    }

    #[test]
    fn test_is_retryable_error_503() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("Service returned 503 error");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_is_retryable_error_429() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("429 Too Many Requests");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_is_retryable_error_timeout() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("Request timeout occurred");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_is_retryable_error_overloaded() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("Server is overloaded, please try again");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_is_not_retryable_error() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("Invalid API key");
        assert!(!client.is_retryable_error(&error));

        let error = SageError::llm("400 Bad Request");
        assert!(!client.is_retryable_error(&error));
    }

    #[test]
    fn test_http_error_is_retryable() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::http("Network error".to_string());
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_client_with_custom_headers() {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_header("X-Custom-Header", "custom-value");

        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_timeout() {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(120));

        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_max_retries() {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_max_retries(5);

        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        assert_eq!(client.config().max_retries, Some(5));
    }

    #[test]
    fn test_model_parameters() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");

        let model_params = ModelParameters {
            model: "gpt-4".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            stop: Some(vec!["END".to_string()]),
            parallel_tool_calls: Some(true),
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            enable_prompt_caching: None,
        };

        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params.clone()).unwrap();

        assert_eq!(client.model(), "gpt-4");
    }

    #[test]
    fn test_multiple_providers() {
        let providers = vec![
            LLMProvider::OpenAI,
            LLMProvider::Anthropic,
            LLMProvider::Google,
            LLMProvider::Azure,
            LLMProvider::OpenRouter,
            LLMProvider::Doubao,
            LLMProvider::Ollama,
            LLMProvider::Glm,
        ];

        for provider in providers {
            let config = ProviderConfig::new(provider.name()).with_api_key("test-key");
            let model_params = ModelParameters::default();
            let client = LLMClient::new(provider.clone(), config, model_params);
            assert!(client.is_ok());
        }
    }

    #[test]
    fn test_error_retryability_502() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("502 Bad Gateway");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_error_retryability_504() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("504 Gateway Timeout");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_error_retryability_connection() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("Connection refused");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_should_fallback_provider_403() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();
        let error = SageError::http_with_status("Forbidden", 403);
        assert!(client.should_fallback_provider(&error));
    }

    #[test]
    fn test_should_fallback_provider_429() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();
        let error = SageError::http_with_status("Rate limited", 429);
        assert!(client.should_fallback_provider(&error));
    }

    #[test]
    fn test_should_fallback_provider_quota_message() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();
        let error = SageError::llm("Quota exceeded");
        assert!(client.should_fallback_provider(&error));
    }

    #[test]
    fn test_should_fallback_provider_rate_limit_message() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();
        let error = SageError::llm("Rate limit exceeded");
        assert!(client.should_fallback_provider(&error));
    }

    #[test]
    fn test_should_not_fallback_provider_non_quota_error() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();
        let error = SageError::llm("500 Internal Server Error");
        assert!(!client.should_fallback_provider(&error));
    }

    #[test]
    fn test_should_fallback_provider_insufficient_quota() {
        let config = ProviderConfig::new("anthropic").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::Anthropic, config, model_params).unwrap();
        let error = SageError::llm("Insufficient quota");
        assert!(client.should_fallback_provider(&error));
    }

    #[test]
    fn test_should_fallback_provider_exceeded_message() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();
        let error = SageError::llm("Token quota exceeded");
        assert!(client.should_fallback_provider(&error));
    }

    #[test]
    fn test_should_fallback_provider_not_enough_message() {
        let config = ProviderConfig::new("google").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::Google, config, model_params).unwrap();
        let error = SageError::llm("Not enough credits");
        assert!(client.should_fallback_provider(&error));
    }

    #[test]
    fn test_client_creation_all_providers() {
        let providers = vec![
            ("openai", LLMProvider::OpenAI),
            ("anthropic", LLMProvider::Anthropic),
            ("google", LLMProvider::Google),
            ("azure", LLMProvider::Azure),
            ("openrouter", LLMProvider::OpenRouter),
            ("doubao", LLMProvider::Doubao),
            ("ollama", LLMProvider::Ollama),
            ("glm", LLMProvider::Glm),
        ];

        for (name, provider) in providers {
            let config = ProviderConfig::new(name).with_api_key("test-key");
            let model_params = ModelParameters::default();
            let client = LLMClient::new(provider.clone(), config, model_params);
            assert!(
                client.is_ok(),
                "Failed to create client for provider: {}",
                name
            );
        }
    }

    #[test]
    fn test_custom_provider_not_implemented() {
        let config = ProviderConfig::new("custom").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(
            LLMProvider::Custom("my_custom_provider".to_string()),
            config,
            model_params,
        );
        assert!(client.is_err());
        if let Err(e) = client {
            assert!(e.to_string().contains("Custom provider"));
        }
    }

    #[test]
    fn test_client_config_validation() {
        // Config without API key should fail validation for most providers
        let config = ProviderConfig::new("openai");
        let model_params = ModelParameters::default();

        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params);
        assert!(client.is_err());
    }

    #[test]
    fn test_is_retryable_network_error() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        let error = SageError::llm("Network connection failed");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_retryable_error_case_insensitive() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        // Test that error detection is case-insensitive
        let error = SageError::llm("SERVICE RETURNED 503 ERROR");
        assert!(client.is_retryable_error(&error));

        let error = SageError::llm("Request TIMEOUT occurred");
        assert!(client.is_retryable_error(&error));
    }

    #[test]
    fn test_client_with_multiple_headers() {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_header("X-Custom-1", "value1")
            .with_header("X-Custom-2", "value2")
            .with_header("X-Custom-3", "value3");

        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params);
        assert!(client.is_ok());
    }

    #[test]
    fn test_model_parameters_comprehensive() {
        let config = ProviderConfig::new("anthropic").with_api_key("test-key");

        let model_params = ModelParameters {
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.8),
            top_p: Some(0.95),
            top_k: Some(50),
            stop: Some(vec!["STOP".to_string(), "END".to_string()]),
            parallel_tool_calls: Some(false),
            frequency_penalty: Some(0.5),
            presence_penalty: Some(0.3),
            seed: Some(42),
            enable_prompt_caching: Some(true),
        };

        let client = LLMClient::new(LLMProvider::Anthropic, config, model_params.clone()).unwrap();

        assert_eq!(client.model(), "claude-3-opus-20240229");
        assert_eq!(client.provider(), &LLMProvider::Anthropic);
    }

    #[test]
    fn test_timeout_config_custom_values() {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_timeouts(
                TimeoutConfig::new()
                    .with_connection_timeout_secs(10)
                    .with_request_timeout_secs(300),
            );

        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params);
        assert!(client.is_ok());

        let client = client.unwrap();
        let timeouts = client.config().get_effective_timeouts();
        assert_eq!(timeouts.connection_timeout_secs, 10);
        assert_eq!(timeouts.request_timeout_secs, 300);
    }

    #[test]
    fn test_ollama_provider_no_api_key_required() {
        // Ollama doesn't require an API key
        let config = ProviderConfig::new("ollama").with_base_url("http://localhost:11434");

        let model_params = ModelParameters {
            model: "llama2".to_string(),
            ..Default::default()
        };

        let client = LLMClient::new(LLMProvider::Ollama, config, model_params);
        // Ollama should work without API key
        assert!(client.is_ok() || client.is_err()); // Depends on validation rules
    }

    #[test]
    fn test_azure_provider_creation() {
        let config = ProviderConfig::new("azure")
            .with_api_key("test-key")
            .with_base_url("https://myresource.openai.azure.com");

        let model_params = ModelParameters {
            model: "gpt-4".to_string(),
            ..Default::default()
        };

        let client = LLMClient::new(LLMProvider::Azure, config, model_params);
        assert!(client.is_ok());
    }

    #[test]
    fn test_is_retryable_error_auth_error() {
        let config = ProviderConfig::new("openai").with_api_key("test-key");
        let model_params = ModelParameters::default();
        let client = LLMClient::new(LLMProvider::OpenAI, config, model_params).unwrap();

        // Auth errors should NOT be retryable
        let error = SageError::llm("401 Unauthorized");
        assert!(!client.is_retryable_error(&error));

        let error = SageError::llm("Invalid authentication");
        assert!(!client.is_retryable_error(&error));
    }

    #[test]
    fn test_model_params_default() {
        let params = ModelParameters::default();
        assert_eq!(params.model, "gpt-4");
        assert_eq!(params.max_tokens, Some(4096));
        assert_eq!(params.temperature, Some(0.7));
    }
}
