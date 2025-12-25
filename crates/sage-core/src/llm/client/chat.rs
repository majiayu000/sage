//! Chat request handling

use super::types::LlmClient;
use crate::error::SageResult;
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::providers::LlmProviderTrait;
use crate::llm::rate_limiter::global as rate_limiter;
use crate::tools::types::ToolSchema;
use tracing::{debug, instrument};

impl LlmClient {
    /// Send a chat completion request.
    ///
    /// Sends a chat completion request to the configured LLM provider with
    /// automatic retry logic and rate limiting.
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history (system, user, assistant messages)
    /// * `tools` - Optional tool schemas for function calling
    ///
    /// # Returns
    ///
    /// Returns the LLM response containing:
    /// - Generated content
    /// - Token usage statistics
    /// - Tool calls (if any)
    /// - Finish reason
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Request fails after all retry attempts
    /// - Provider returns an error response
    /// - Network connectivity issues
    /// - API key is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::llm::client::LlmClient;
    /// use sage_core::llm::messages::LlmMessage;
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
    /// let messages = vec![
    ///     LlmMessage::system("You are a helpful assistant."),
    ///     LlmMessage::user("What is the capital of France?"),
    /// ];
    ///
    /// let response = client.chat(&messages, None).await?;
    /// println!("Assistant: {}", response.content);
    /// println!("Tokens used: {:?}", response.usage);
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, messages, tools), fields(provider = %self.provider, model = %self.model_params.model))]
    pub async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        // Apply rate limiting before making the request
        let provider_name = self.provider.name();
        let limiter = rate_limiter::get_rate_limiter(provider_name).await;

        if let Some(wait_duration) = limiter.acquire().await {
            debug!(
                "Rate limited for provider '{}', waited {:.2}s",
                provider_name,
                wait_duration.as_secs_f64()
            );
        }

        // Execute the request with retry logic
        self.execute_with_retry(|| async { self.provider_instance.chat(messages, tools).await })
            .await
    }
}
