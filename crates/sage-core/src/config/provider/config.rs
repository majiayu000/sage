//! Main ProviderConfig struct

use super::api_key::{get_standard_env_vars, ApiKeyInfo, ApiKeySource};
use super::auth::ApiAuthConfig;
use super::network::NetworkConfig;
use super::resilience::{RateLimitConfig, ResilienceConfig};
use crate::llm::provider_types::TimeoutConfig;
use serde::{Deserialize, Serialize};

/// Configuration for a specific LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (openai, anthropic, etc.)
    pub name: String,
    /// API version (used by some providers like Anthropic)
    pub api_version: Option<String>,
    /// Authentication configuration
    #[serde(flatten)]
    pub auth: ApiAuthConfig,
    /// Network configuration
    #[serde(flatten)]
    pub network: NetworkConfig,
    /// Resilience configuration (retry/rate limiting)
    #[serde(flatten)]
    pub resilience: ResilienceConfig,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: "openai".to_string(),
            api_version: None,
            auth: ApiAuthConfig::default(),
            network: NetworkConfig::default(),
            resilience: ResilienceConfig::new(),
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
        self.auth.api_key = Some(api_key.into());
        self
    }

    /// Set organization ID (for OpenAI)
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.auth.organization = Some(org.into());
        self
    }

    /// Set project ID
    pub fn with_project_id(mut self, project: impl Into<String>) -> Self {
        self.auth.project_id = Some(project.into());
        self
    }

    /// Set base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.network.base_url = Some(base_url.into());
        self
    }

    /// Set API version
    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = Some(api_version.into());
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.network.headers.insert(key.into(), value.into());
        self
    }

    /// Set timeout configuration
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.network.timeouts = timeouts;
        self
    }

    /// Set legacy timeout (deprecated)
    #[deprecated(since = "0.1.0", note = "Use with_timeouts instead")]
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.network.timeouts.request_timeout_secs = timeout;
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.resilience.max_retries = Some(max_retries);
        self
    }

    /// Set rate limiting
    pub fn with_rate_limit(mut self, rate_limit: RateLimitConfig) -> Self {
        self.resilience.rate_limit = Some(rate_limit);
        self
    }

    /// Set authentication configuration
    pub fn with_auth(mut self, auth: ApiAuthConfig) -> Self {
        self.auth = auth;
        self
    }

    /// Set network configuration
    pub fn with_network(mut self, network: NetworkConfig) -> Self {
        self.network = network;
        self
    }

    /// Set resilience configuration
    pub fn with_resilience(mut self, resilience: ResilienceConfig) -> Self {
        self.resilience = resilience;
        self
    }

    /// Get the effective base URL for this provider
    pub fn get_base_url(&self) -> String {
        if let Some(base_url) = &self.network.base_url {
            base_url.clone()
        } else {
            match self.name.as_str() {
                "openai" => "https://api.openai.com/v1".to_string(),
                "anthropic" => "https://api.anthropic.com".to_string(),
                "google" => "https://generativelanguage.googleapis.com".to_string(),
                "ollama" => "http://localhost:11434".to_string(),
                "glm" | "zhipu" => "https://open.bigmodel.cn/api/anthropic".to_string(),
                _ => "http://localhost:8000".to_string(),
            }
        }
    }

    /// Get the effective API key (from config or environment)
    pub fn get_api_key(&self) -> Option<String> {
        self.get_api_key_info().key
    }

    /// Get detailed API key information including source
    pub fn get_api_key_info(&self) -> ApiKeyInfo {
        let provider_upper = self.name.to_uppercase();

        // 1. Try SAGE-prefixed env var first
        let sage_env_var = format!("SAGE_{}_API_KEY", provider_upper);
        if let Ok(key) = std::env::var(&sage_env_var) {
            if !key.is_empty() {
                return ApiKeyInfo {
                    key: Some(key),
                    source: ApiKeySource::SageEnvVar,
                    env_var_name: Some(sage_env_var),
                };
            }
        }

        // 2. Try standard environment variables
        for env_var in get_standard_env_vars(&self.name) {
            if let Ok(key) = std::env::var(&env_var) {
                if !key.is_empty() {
                    return ApiKeyInfo {
                        key: Some(key),
                        source: ApiKeySource::StandardEnvVar,
                        env_var_name: Some(env_var),
                    };
                }
            }
        }

        // 3. Fall back to config file
        if let Some(api_key) = &self.auth.api_key {
            if !api_key.is_empty() {
                return ApiKeyInfo {
                    key: Some(api_key.clone()),
                    source: ApiKeySource::ConfigFile,
                    env_var_name: None,
                };
            }
        }

        ApiKeyInfo {
            key: None,
            source: ApiKeySource::NotFound,
            env_var_name: None,
        }
    }

    /// Check if this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        !matches!(self.name.as_str(), "ollama")
    }

    /// Validate the API key format for this provider
    pub fn validate_api_key_format(&self) -> Result<(), String> {
        let key_info = self.get_api_key_info();

        if !self.requires_api_key() {
            return Ok(());
        }

        let key = match &key_info.key {
            Some(k) => k,
            None => {
                return Err(format!(
                    "API key required for '{}'. Set via {} or config file",
                    self.name,
                    get_standard_env_vars(&self.name)
                        .first()
                        .cloned()
                        .unwrap_or_default()
                ));
            }
        };

        match self.name.as_str() {
            "anthropic" => {
                if !key.starts_with("sk-ant-") {
                    return Err("Anthropic API key should start with 'sk-ant-'".to_string());
                }
            }
            "openai" => {
                if !key.starts_with("sk-") {
                    return Err("OpenAI API key should start with 'sk-'".to_string());
                }
            }
            "google" => {
                if key.len() < 20 {
                    return Err("Google API key appears too short".to_string());
                }
            }
            "glm" => {
                if key.len() < 10 {
                    return Err("GLM API key appears too short".to_string());
                }
            }
            _ => {
                if key.is_empty() || key.contains("your-") || key.contains("xxx") {
                    return Err("API key appears to be a placeholder".to_string());
                }
            }
        }

        Ok(())
    }
}
