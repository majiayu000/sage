//! Streaming chat support

use super::types::LlmClient;
use crate::error::{SageError, SageResult};
use crate::llm::messages::LlmMessage;
use crate::llm::providers::LlmProviderTrait;
use crate::llm::rate_limiter::global as rate_limiter;
use crate::llm::streaming::{LlmStream, StreamingLlmClient};
use crate::recovery::circuit_breaker::CircuitBreakerError;
use crate::tools::types::ToolSchema;
use async_trait::async_trait;
use tracing::{debug, instrument, warn};

#[async_trait]
impl StreamingLlmClient for LlmClient {
    /// Send a streaming chat completion request.
    ///
    /// Initiates a streaming response from the LLM provider, allowing you to
    /// process generated tokens as they arrive rather than waiting for the
    /// complete response.
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history (system, user, assistant messages)
    /// * `tools` - Optional tool schemas for function calling
    ///
    /// # Returns
    ///
    /// Returns an `LlmStream` that yields chunks of the response as they're generated.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Stream initialization fails
    /// - Provider doesn't support streaming
    /// - Network connectivity issues
    /// - API key is invalid
    /// - Circuit breaker is open (too many recent failures)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::messages::LlmMessage;
    /// use sage_core::llm::streaming::StreamingLlmClient;
    /// use futures::StreamExt;
    /// # use sage_core::llm::provider_types::LlmProvider;
    /// # use sage_core::config::provider::ProviderConfig;
    /// # use sage_core::llm::provider_types::ModelParameters;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LlmClient::new(
    ///     LlmProvider::Anthropic,
    ///     ProviderConfig::default(),
    ///     ModelParameters::default()
    /// )?;
    ///
    /// let messages = vec![LlmMessage::user("Tell me a story")];
    /// let mut stream = client.chat_stream(&messages, None).await?;
    ///
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(response) => {
    ///             if let Some(content) = response.content {
    ///                 print!("{}", content);
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Stream error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, messages, tools), fields(provider = %self.provider, model = %self.model_params.model))]
    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        // Apply rate limiting before making the request
        let provider_name = self.provider.name();
        let limiter = rate_limiter::get_rate_limiter(provider_name).await;

        if let Some(wait_duration) = limiter.acquire().await {
            debug!(
                "Rate limited for provider '{}' (streaming), waited {:.2}s",
                provider_name,
                wait_duration.as_secs_f64()
            );
        }

        // Execute the streaming request with circuit breaker protection
        let result = self
            .circuit_breaker
            .call(|| async { self.provider_instance.chat_stream(messages, tools).await })
            .await;

        // Convert circuit breaker errors to SageError
        match result {
            Ok(stream) => {
                tracing::info!("streaming request initiated");
                Ok(stream)
            }
            Err(CircuitBreakerError::Open { component }) => {
                warn!(
                    "Circuit breaker open for '{}' (streaming), rejecting request",
                    component
                );
                Err(SageError::llm_with_provider(
                    format!(
                        "Service temporarily unavailable: circuit breaker open for {}. Too many recent failures.",
                        component
                    ),
                    provider_name,
                ))
            }
            Err(CircuitBreakerError::OperationFailed(e)) => Err(e),
        }
    }
}
