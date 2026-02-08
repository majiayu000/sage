//! Provider trait and unified enum

use crate::error::SageResult;
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::streaming::LlmStream;
use crate::tools::types::ToolSchema;
use async_trait::async_trait;

/// Unified trait for all LLM providers
#[async_trait]
pub trait LlmProviderTrait: Send + Sync {
    /// Send a chat completion request
    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse>;

    /// Send a streaming chat completion request
    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream>;
}

/// Unified provider enum that wraps all provider implementations
pub enum ProviderInstance {
    OpenAI(super::OpenAiProvider),
    Anthropic(super::AnthropicProvider),
    Google(super::GoogleProvider),
    Azure(super::AzureProvider),
    OpenRouter(super::OpenRouterProvider),
    Ollama(super::OllamaProvider),
    Doubao(super::DoubaoProvider),
    Glm(super::GlmProvider),
}

/// Dispatch a method call to the inner provider for all variants
macro_rules! dispatch_provider {
    ($self:expr, $method:ident($($arg:expr),*)) => {
        match $self {
            Self::OpenAI(p) => p.$method($($arg),*).await,
            Self::Anthropic(p) => p.$method($($arg),*).await,
            Self::Google(p) => p.$method($($arg),*).await,
            Self::Azure(p) => p.$method($($arg),*).await,
            Self::OpenRouter(p) => p.$method($($arg),*).await,
            Self::Ollama(p) => p.$method($($arg),*).await,
            Self::Doubao(p) => p.$method($($arg),*).await,
            Self::Glm(p) => p.$method($($arg),*).await,
        }
    };
}

#[async_trait]
impl LlmProviderTrait for ProviderInstance {
    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        dispatch_provider!(self, chat(messages, tools))
    }

    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        dispatch_provider!(self, chat_stream(messages, tools))
    }
}
