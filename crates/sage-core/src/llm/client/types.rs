//! LLM client type definitions

use crate::config::provider::ProviderConfig;
use crate::llm::provider_types::{LlmProvider, LlmRequestParams};
use crate::llm::providers::ProviderInstance;
use crate::recovery::circuit_breaker::CircuitBreaker;
use std::sync::Arc;

/// LLM client for making requests to various providers.
///
/// Provides a unified interface for interacting with multiple LLM providers
/// (OpenAI, Anthropic, Google, Azure, etc.) with automatic retry logic,
/// rate limiting, and streaming support.
///
/// # Features
///
/// - **Multi-provider support**: OpenAI, Anthropic, Google, Azure, OpenRouter, Ollama, Doubao, GLM
/// - **Automatic retries**: Exponential backoff with jitter for transient failures
/// - **Rate limiting**: Global rate limiter prevents hitting API limits
/// - **Streaming**: Support for streaming responses via `StreamingLlmClient` trait
/// - **Circuit breaker**: Protection against cascading failures
/// - **Custom headers**: Support for custom HTTP headers
///
/// # Examples
///
/// ```no_run
/// use sage_core::llm::client::LlmClient;
/// use sage_core::llm::provider_types::{LlmProvider, LlmRequestParams};
/// use sage_core::config::provider::ProviderConfig;
/// use sage_core::llm::messages::LlmMessage;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create client for Anthropic
/// let provider = LlmProvider::Anthropic;
/// let config = ProviderConfig::default();
/// let params = LlmRequestParams {
///     model: "claude-3-5-sonnet-20241022".to_string(),
///     ..Default::default()
/// };
///
/// let client = LlmClient::new(provider, config, params)?;
///
/// // Send a chat request
/// let messages = vec![LlmMessage::user("Hello, world!")];
/// let response = client.chat(&messages, None).await?;
/// println!("Response: {}", response.content);
/// # Ok(())
/// # }
/// ```
pub struct LlmClient {
    pub(super) provider: LlmProvider,
    pub(super) config: ProviderConfig,
    pub(super) model_params: LlmRequestParams,
    pub(super) provider_instance: ProviderInstance,
    /// Circuit breaker for protecting against cascading failures
    pub(super) circuit_breaker: Arc<CircuitBreaker>,
}
