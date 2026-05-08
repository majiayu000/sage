//! Provider-specific default `ProviderConfig`.
//!
//! `id` and `base_url` are derived from the canonical
//! [`embedded_providers()`][crate::config::embedded_providers] list.
//! Per-provider tuning that is not part of the registry contract
//! (timeouts, retries, rate limits, api_version) lives here.
//!
//! Closes #23 alias drift: `zhipu` and `kimi` are still accepted at
//! the dispatch level so existing user configs keep working — they
//! route to the canonical provider — but the URLs they resolve to
//! come from the registry rather than from a duplicated literal.

use super::config::ProviderConfig;
use super::resilience::RateLimitConfig;
use crate::llm::provider_types::TimeoutConfig;

pub struct ProviderDefaults;

impl ProviderDefaults {
    pub fn openai() -> ProviderConfig {
        Self::with_registry_url("openai")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("openai"))
    }

    pub fn zai() -> ProviderConfig {
        Self::with_registry_url("zai")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("zai"))
    }

    pub fn anthropic() -> ProviderConfig {
        Self::with_registry_url("anthropic")
            .with_api_version("2023-06-01")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("anthropic"))
    }

    pub fn google() -> ProviderConfig {
        Self::with_registry_url("google")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("google"))
    }

    pub fn ollama() -> ProviderConfig {
        Self::with_registry_url("ollama")
            .with_timeouts(
                TimeoutConfig::new()
                    .with_connection_timeout_secs(10)
                    .with_request_timeout_secs(120),
            )
            .with_max_retries(1)
            .with_rate_limit(RateLimitConfig::for_provider("ollama"))
    }

    pub fn glm() -> ProviderConfig {
        Self::with_registry_url("glm")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("glm"))
    }

    pub fn moonshot() -> ProviderConfig {
        Self::with_registry_url("moonshot")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("moonshot"))
    }

    /// Azure has no `api_base_url` in the registry — the user must
    /// supply their own resource URL — so we don't call
    /// `with_registry_url` here.
    pub fn azure() -> ProviderConfig {
        ProviderConfig::new("azure")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("azure"))
    }

    pub fn openrouter() -> ProviderConfig {
        Self::with_registry_url("openrouter")
            .with_timeouts(
                TimeoutConfig::new()
                    .with_connection_timeout_secs(30)
                    .with_request_timeout_secs(90),
            )
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("openrouter"))
    }

    /// Doubao is not (yet) in the embedded registry.
    pub fn doubao() -> ProviderConfig {
        ProviderConfig::new("doubao")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig::for_provider("doubao"))
    }

    pub fn for_provider(name: &str) -> ProviderConfig {
        match name {
            "openai" => Self::openai(),
            "zai" => Self::zai(),
            "anthropic" => Self::anthropic(),
            "google" => Self::google(),
            "azure" => Self::azure(),
            "openrouter" => Self::openrouter(),
            "doubao" => Self::doubao(),
            "ollama" => Self::ollama(),
            "glm" | "zhipu" => Self::glm(),
            "moonshot" | "kimi" => Self::moonshot(),
            _ => ProviderConfig::new(name),
        }
    }

    fn with_registry_url(id: &str) -> ProviderConfig {
        let mut cfg = ProviderConfig::new(id);
        if let Some(url) = registry_base_url(id) {
            cfg = cfg.with_base_url(url);
        }
        cfg
    }
}

fn registry_base_url(id: &str) -> Option<String> {
    crate::config::embedded_providers::embedded_providers()
        .into_iter()
        .find(|p| p.id == id)
        .and_then(|p| {
            if p.api_base_url.is_empty() {
                None
            } else {
                Some(p.api_base_url)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_base_url_matches_embedded_providers() {
        // Regression for issue #23: previously OpenAI's URL was
        // hardcoded as `https://api.openai.com` here while the
        // registry said `/v1`. They must agree.
        for info in crate::config::embedded_providers::embedded_providers() {
            if info.api_base_url.is_empty() {
                continue;
            }
            assert_eq!(
                registry_base_url(&info.id).as_deref(),
                Some(info.api_base_url.as_str()),
                "registry_base_url drift for {:?}",
                info.id
            );
        }
    }

    #[test]
    fn openai_uses_v1_path() {
        let cfg = ProviderDefaults::openai();
        assert_eq!(
            cfg.network.base_url.as_deref(),
            Some("https://api.openai.com/v1")
        );
    }

    #[test]
    fn aliases_route_to_canonical_provider() {
        let zhipu = ProviderDefaults::for_provider("zhipu");
        let glm = ProviderDefaults::for_provider("glm");
        assert_eq!(zhipu.network.base_url, glm.network.base_url);

        let kimi = ProviderDefaults::for_provider("kimi");
        let moonshot = ProviderDefaults::for_provider("moonshot");
        assert_eq!(kimi.network.base_url, moonshot.network.base_url);
    }
}
