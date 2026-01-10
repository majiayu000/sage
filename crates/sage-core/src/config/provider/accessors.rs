//! ProviderConfig accessor methods and validation

use super::api_key::get_standard_env_vars;
use super::config::ProviderConfig;
use super::resilience::RateLimitConfig;
use crate::llm::provider_types::TimeoutConfig;
use std::collections::HashMap;

impl ProviderConfig {
    /// Get the effective timeout configuration
    pub fn get_effective_timeouts(&self) -> TimeoutConfig {
        self.network.get_effective_timeouts()
    }

    /// Validate the provider configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Provider name cannot be empty".to_string());
        }

        if self.requires_api_key() && self.get_api_key().is_none() {
            return Err(format!(
                "API key is required for provider '{}'. Set it in config or environment variables",
                self.name
            ));
        }

        let effective_timeouts = self.get_effective_timeouts();
        effective_timeouts.validate()?;

        if let Some(max_retries) = self.resilience.max_retries {
            if max_retries > 10 {
                return Err("Max retries should not exceed 10".to_string());
            }
        }

        Ok(())
    }

    /// Get API key from auth config (direct access)
    #[inline]
    pub fn api_key(&self) -> Option<&String> {
        self.auth.api_key.as_ref()
    }

    /// Get organization from auth config (direct access)
    #[inline]
    pub fn organization(&self) -> Option<&String> {
        self.auth.organization.as_ref()
    }

    /// Get project ID from auth config (direct access)
    #[inline]
    pub fn project_id(&self) -> Option<&String> {
        self.auth.project_id.as_ref()
    }

    /// Get base URL from network config (direct access)
    #[inline]
    pub fn base_url(&self) -> Option<&String> {
        self.network.base_url.as_ref()
    }

    /// Get headers from network config (direct access)
    #[inline]
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.network.headers
    }

    /// Get mutable headers from network config
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.network.headers
    }

    /// Get timeouts from network config (direct access)
    #[inline]
    pub fn timeouts(&self) -> TimeoutConfig {
        self.network.timeouts
    }

    /// Get max retries from resilience config (direct access)
    #[inline]
    pub fn max_retries(&self) -> Option<u32> {
        self.resilience.max_retries
    }

    /// Get rate limit from resilience config (direct access)
    #[inline]
    pub fn rate_limit(&self) -> Option<&RateLimitConfig> {
        self.resilience.rate_limit.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::super::api_key::ApiKeySource;
    use super::*;

    #[test]
    fn test_provider_config_api_key_from_config() {
        let config = ProviderConfig::new("anthropic").with_api_key("sk-ant-test-key-12345");
        let info = config.get_api_key_info();
        assert_eq!(info.source, ApiKeySource::ConfigFile);
        assert!(info.key.is_some());
    }

    #[test]
    fn test_provider_requires_api_key() {
        assert!(ProviderConfig::new("anthropic").requires_api_key());
        assert!(ProviderConfig::new("openai").requires_api_key());
        assert!(!ProviderConfig::new("ollama").requires_api_key());
    }
}
