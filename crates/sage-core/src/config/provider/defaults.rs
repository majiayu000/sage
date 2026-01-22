//! Provider-specific default configurations

use super::config::ProviderConfig;
use super::resilience::RateLimitConfig;
use crate::llm::provider_types::TimeoutConfig;

/// Provider-specific default configurations
pub struct ProviderDefaults;

impl ProviderDefaults {
    /// Get default configuration for OpenAI
    pub fn openai() -> ProviderConfig {
        ProviderConfig::new("openai")
            .with_base_url("https://api.openai.com/v1")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("openai"))
    }

    /// Get default configuration for Anthropic
    pub fn anthropic() -> ProviderConfig {
        ProviderConfig::new("anthropic")
            .with_base_url("https://api.anthropic.com")
            .with_api_version("2023-06-01")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("anthropic"))
    }

    /// Get default configuration for Google
    pub fn google() -> ProviderConfig {
        ProviderConfig::new("google")
            .with_base_url("https://generativelanguage.googleapis.com")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("google"))
    }

    /// Get default configuration for Ollama (local models)
    pub fn ollama() -> ProviderConfig {
        ProviderConfig::new("ollama")
            .with_base_url("http://localhost:11434")
            .with_timeouts(
                TimeoutConfig::new()
                    .with_connection_timeout_secs(10)
                    .with_request_timeout_secs(120),
            )
            .with_max_retries(1)
            .with_rate_limit(RateLimitConfig::for_provider("ollama"))
    }

    /// Get default configuration for GLM (Zhipu AI)
    pub fn glm() -> ProviderConfig {
        ProviderConfig::new("glm")
            .with_base_url("https://open.bigmodel.cn/api/anthropic")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("glm"))
    }

    /// Get default configuration for Azure OpenAI
    pub fn azure() -> ProviderConfig {
        ProviderConfig::new("azure")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("azure"))
    }

    /// Get default configuration for OpenRouter
    pub fn openrouter() -> ProviderConfig {
        ProviderConfig::new("openrouter")
            .with_base_url("https://openrouter.ai")
            .with_timeouts(
                TimeoutConfig::new()
                    .with_connection_timeout_secs(30)
                    .with_request_timeout_secs(90),
            )
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("openrouter"))
    }

    /// Get default configuration for Doubao
    pub fn doubao() -> ProviderConfig {
        ProviderConfig::new("doubao")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("doubao"))
    }

    /// Get default configuration for a provider by name
    pub fn for_provider(name: &str) -> ProviderConfig {
        match name {
            "openai" => Self::openai(),
            "anthropic" => Self::anthropic(),
            "google" => Self::google(),
            "azure" => Self::azure(),
            "openrouter" => Self::openrouter(),
            "doubao" => Self::doubao(),
            "ollama" => Self::ollama(),
            "glm" | "zhipu" => Self::glm(),
            _ => ProviderConfig::new(name),
        }
    }
}
