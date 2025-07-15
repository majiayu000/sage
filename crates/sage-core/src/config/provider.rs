//! Provider-specific configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a specific LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (openai, anthropic, etc.)
    pub name: String,
    /// API endpoint base URL
    pub base_url: Option<String>,
    /// API key
    pub api_key: Option<String>,
    /// API version
    pub api_version: Option<String>,
    /// Organization ID (for OpenAI)
    pub organization: Option<String>,
    /// Project ID
    pub project_id: Option<String>,
    /// Custom headers
    pub headers: HashMap<String, String>,
    /// Request timeout in seconds
    pub timeout: Option<u64>,
    /// Maximum number of retries
    pub max_retries: Option<u32>,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: "openai".to_string(),
            base_url: None,
            api_key: None,
            api_version: None,
            organization: None,
            project_id: None,
            headers: HashMap::new(),
            timeout: Some(60),
            max_retries: Some(3),
            rate_limit: None,
        }
    }
}

impl ProviderConfig {
    /// Create a new provider config
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Set API version
    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = Some(api_version.into());
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Set rate limiting
    pub fn with_rate_limit(mut self, rate_limit: RateLimitConfig) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Get the effective base URL for this provider
    pub fn get_base_url(&self) -> String {
        if let Some(base_url) = &self.base_url {
            base_url.clone()
        } else {
            match self.name.as_str() {
                "openai" => "https://api.openai.com/v1".to_string(),
                "anthropic" => "https://api.anthropic.com".to_string(),
                "google" => "https://generativelanguage.googleapis.com".to_string(),
                "ollama" => "http://localhost:11434".to_string(),
                _ => "http://localhost:8000".to_string(),
            }
        }
    }

    /// Get the effective API key (from config or environment)
    pub fn get_api_key(&self) -> Option<String> {
        if let Some(api_key) = &self.api_key {
            return Some(api_key.clone());
        }

        // Try environment variables based on provider
        match self.name.as_str() {
            "openai" => std::env::var("OPENAI_API_KEY").ok(),
            "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
            "google" => std::env::var("GOOGLE_API_KEY").ok(),
            _ => None,
        }
    }

    /// Check if this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        matches!(self.name.as_str(), "openai" | "anthropic" | "google")
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

        if let Some(timeout) = self.timeout {
            if timeout == 0 {
                return Err("Timeout must be greater than 0".to_string());
            }
        }

        if let Some(max_retries) = self.max_retries {
            if max_retries > 10 {
                return Err("Max retries should not exceed 10".to_string());
            }
        }

        Ok(())
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: Option<u32>,
    /// Maximum tokens per minute
    pub tokens_per_minute: Option<u32>,
    /// Maximum concurrent requests
    pub max_concurrent_requests: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: Some(60),
            tokens_per_minute: Some(100_000),
            max_concurrent_requests: Some(10),
        }
    }
}

/// Provider-specific default configurations
pub struct ProviderDefaults;

impl ProviderDefaults {
    /// Get default configuration for OpenAI
    pub fn openai() -> ProviderConfig {
        ProviderConfig::new("openai")
            .with_base_url("https://api.openai.com/v1")
            .with_timeout(60)
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(100_000),
                max_concurrent_requests: Some(10),
            })
    }

    /// Get default configuration for Anthropic
    pub fn anthropic() -> ProviderConfig {
        ProviderConfig::new("anthropic")
            .with_base_url("https://api.anthropic.com")
            .with_api_version("2023-06-01")
            .with_timeout(60)
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(50),
                tokens_per_minute: Some(80_000),
                max_concurrent_requests: Some(5),
            })
    }

    /// Get default configuration for Google
    pub fn google() -> ProviderConfig {
        ProviderConfig::new("google")
            .with_base_url("https://generativelanguage.googleapis.com")
            .with_timeout(60)
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(120_000),
                max_concurrent_requests: Some(10),
            })
    }

    /// Get default configuration for Ollama
    pub fn ollama() -> ProviderConfig {
        ProviderConfig::new("ollama")
            .with_base_url("http://localhost:11434")
            .with_timeout(120) // Longer timeout for local models
            .with_max_retries(1)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: None, // No rate limiting for local
                tokens_per_minute: None,
                max_concurrent_requests: Some(1), // Usually one at a time for local
            })
    }

    /// Get default configuration for a provider by name
    pub fn for_provider(name: &str) -> ProviderConfig {
        match name {
            "openai" => Self::openai(),
            "anthropic" => Self::anthropic(),
            "google" => Self::google(),
            "ollama" => Self::ollama(),
            _ => ProviderConfig::new(name),
        }
    }
}
