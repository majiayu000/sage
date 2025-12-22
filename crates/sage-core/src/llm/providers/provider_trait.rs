//! Provider trait and unified enum

use crate::error::SageResult;
use crate::llm::messages::{LLMMessage, LLMResponse};
use crate::llm::streaming::LLMStream;
use crate::tools::types::ToolSchema;
use async_trait::async_trait;

/// Unified trait for all LLM providers
#[async_trait]
pub trait LLMProviderTrait: Send + Sync {
    /// Send a chat completion request
    async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse>;

    /// Send a streaming chat completion request
    async fn chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream>;
}

/// Unified provider enum that wraps all provider implementations
pub enum ProviderInstance {
    OpenAI(super::OpenAIProvider),
    Anthropic(super::AnthropicProvider),
    Google(super::GoogleProvider),
    Azure(super::AzureProvider),
    OpenRouter(super::OpenRouterProvider),
    Ollama(super::OllamaProvider),
    Doubao(super::DoubaoProvider),
    Glm(super::GlmProvider),
}

#[async_trait]
impl LLMProviderTrait for ProviderInstance {
    async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        match self {
            Self::OpenAI(p) => p.chat(messages, tools).await,
            Self::Anthropic(p) => p.chat(messages, tools).await,
            Self::Google(p) => p.chat(messages, tools).await,
            Self::Azure(p) => p.chat(messages, tools).await,
            Self::OpenRouter(p) => p.chat(messages, tools).await,
            Self::Ollama(p) => p.chat(messages, tools).await,
            Self::Doubao(p) => p.chat(messages, tools).await,
            Self::Glm(p) => p.chat(messages, tools).await,
        }
    }

    async fn chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        match self {
            Self::OpenAI(p) => p.chat_stream(messages, tools).await,
            Self::Anthropic(p) => p.chat_stream(messages, tools).await,
            Self::Google(p) => p.chat_stream(messages, tools).await,
            Self::Azure(p) => p.chat_stream(messages, tools).await,
            Self::OpenRouter(p) => p.chat_stream(messages, tools).await,
            Self::Ollama(p) => p.chat_stream(messages, tools).await,
            Self::Doubao(p) => p.chat_stream(messages, tools).await,
            Self::Glm(p) => p.chat_stream(messages, tools).await,
        }
    }
}
