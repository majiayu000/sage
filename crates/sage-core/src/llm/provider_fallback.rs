//! Provider-level fallback for quota and rate limit errors

use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::llm::client::LLMClient;
use crate::llm::messages::{LLMMessage, LLMResponse};
use crate::llm::provider_types::{LLMProvider, ModelParameters};
use crate::tools::types::ToolSchema;
use tracing::{info, warn};

/// Provider fallback client that switches providers on quota errors
pub struct ProviderFallbackClient {
    clients: Vec<LLMClient>,
    current_index: usize,
}

impl ProviderFallbackClient {
    /// Create a new provider fallback client
    pub fn new(providers: Vec<(LLMProvider, ProviderConfig, ModelParameters)>) -> SageResult<Self> {
        let mut clients = Vec::new();
        for (provider, config, params) in providers {
            clients.push(LLMClient::new(provider, config, params)?);
        }

        if clients.is_empty() {
            return Err(SageError::config("No providers configured for fallback"));
        }

        Ok(Self {
            clients,
            current_index: 0,
        })
    }

    /// Send a chat request with automatic provider fallback
    pub async fn chat(
        &mut self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let mut last_error = None;

        for attempt in 0..self.clients.len() {
            let client = &self.clients[self.current_index];

            match client.chat(messages, tools).await {
                Ok(response) => {
                    if attempt > 0 {
                        info!(
                            "Successfully fell back to provider: {}",
                            client.provider().name()
                        );
                    }
                    return Ok(response);
                }
                Err(error) => {
                    last_error = Some(error.clone());

                    if client.should_fallback_provider(&error) {
                        warn!(
                            "Provider {} quota/rate limit error: {}. Falling back...",
                            client.provider().name(),
                            error
                        );

                        self.current_index = (self.current_index + 1) % self.clients.len();
                        continue;
                    }

                    return Err(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SageError::llm("All providers exhausted")))
    }

    /// Get current provider name
    pub fn current_provider(&self) -> &str {
        self.clients[self.current_index].provider().name()
    }
}
