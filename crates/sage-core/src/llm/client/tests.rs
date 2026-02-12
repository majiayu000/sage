//! Tests for LLM client with circuit breaker integration

#[cfg(test)]
mod tests {
    use crate::config::provider::ProviderConfig;
    use crate::llm::client::LlmClient;
    use crate::llm::provider_types::{LlmProvider, LlmRequestParams};
    use crate::recovery::circuit_breaker::CircuitState;

    #[test]
    fn test_client_creation_with_circuit_breaker() {
        let client = LlmClient::new(
            LlmProvider::OpenAI,
            ProviderConfig::new("openai").with_api_key("test-key"),
            LlmRequestParams::default(),
        )
        .expect("Client should be created");

        assert!(matches!(client.provider(), LlmProvider::OpenAI));
    }

    #[tokio::test]
    async fn test_circuit_breaker_stats_initial_state() {
        let client = LlmClient::new(
            LlmProvider::OpenAI,
            ProviderConfig::new("openai").with_api_key("test-key"),
            LlmRequestParams::default(),
        )
        .expect("Client should be created");

        let stats = client.circuit_breaker_stats().await;
        assert_eq!(stats.state, CircuitState::Closed);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.total_calls, 0);
    }

    #[tokio::test]
    async fn test_is_circuit_open_initial_state() {
        let client = LlmClient::new(
            LlmProvider::OpenAI,
            ProviderConfig::new("openai").with_api_key("test-key"),
            LlmRequestParams::default(),
        )
        .expect("Client should be created");

        assert!(!client.is_circuit_open().await);
    }

    #[tokio::test]
    async fn test_reset_circuit_breaker() {
        let client = LlmClient::new(
            LlmProvider::OpenAI,
            ProviderConfig::new("openai").with_api_key("test-key"),
            LlmRequestParams::default(),
        )
        .expect("Client should be created");

        // Reset should work even when already closed
        client.reset_circuit_breaker().await;

        let stats = client.circuit_breaker_stats().await;
        assert_eq!(stats.state, CircuitState::Closed);
    }

    #[test]
    fn test_client_with_different_providers() {
        let providers = vec![
            LlmProvider::OpenAI,
            LlmProvider::Anthropic,
            LlmProvider::Google,
            LlmProvider::Azure,
            LlmProvider::Ollama,
        ];

        for provider in providers {
            let config = ProviderConfig::new(provider.name()).with_api_key("test-key");
            let client = LlmClient::new(provider.clone(), config, LlmRequestParams::default());
            assert!(
                client.is_ok(),
                "Failed to create client for provider: {:?}",
                provider
            );
        }
    }
}
