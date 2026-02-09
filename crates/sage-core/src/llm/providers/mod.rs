//! Provider-specific implementations

pub mod anthropic;
pub mod anthropic_stream;
pub mod azure;
pub mod doubao;
pub mod error_utils;
pub mod glm;
pub mod google;
pub mod ollama;
pub mod openai;
pub mod openai_stream;
pub mod openrouter;
pub mod provider_trait;
pub mod request_builder;

#[cfg(test)]
mod openai_tests;

pub use anthropic::AnthropicProvider;
pub use azure::AzureProvider;
pub use doubao::DoubaoProvider;
pub use glm::GlmProvider;
pub use google::GoogleProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use provider_trait::{LlmProviderTrait, ProviderInstance};
