//! Tests for provider fallback

#[cfg(test)]
mod tests {
    use crate::config::provider::ProviderConfig;
    use crate::llm::provider_fallback::ProviderFallbackClient;
    use crate::llm::provider_types::{LlmProvider, LlmRequestParams};

    #[test]
    fn test_provider_fallback_creation() {
        let providers = vec![
            (
                LlmProvider::Google,
                ProviderConfig::new("google").with_api_key("test_key"),
                LlmRequestParams {
                    model: "gemini-pro".to_string(),
                    temperature: Some(0.7),
                    max_tokens: Some(4096),
                    top_p: None,
                    top_k: None,
                    stop: None,
                    parallel_tool_calls: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    seed: None,
                    enable_prompt_caching: Some(false),
                },
            ),
            (
                LlmProvider::Anthropic,
                ProviderConfig::new("anthropic").with_api_key("test_key"),
                LlmRequestParams {
                    model: "claude-3-5-sonnet-20241022".to_string(),
                    temperature: Some(0.7),
                    max_tokens: Some(4096),
                    top_p: None,
                    top_k: None,
                    stop: None,
                    parallel_tool_calls: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    seed: None,
                    enable_prompt_caching: Some(false),
                },
            ),
        ];

        let result = ProviderFallbackClient::new(providers);
        assert!(result.is_ok());

        let client = result.unwrap();
        assert_eq!(client.current_provider(), "google");
    }

    #[test]
    fn test_empty_providers() {
        let result = ProviderFallbackClient::new(vec![]);
        assert!(result.is_err());
    }
}
