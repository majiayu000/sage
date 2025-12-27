//! Provider-specific configuration

use crate::llm::provider_types::TimeoutConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source of the API key
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeySource {
    /// From configuration file
    ConfigFile,
    /// From SAGE_<PROVIDER>_API_KEY environment variable
    SageEnvVar,
    /// From standard environment variable (e.g., ANTHROPIC_API_KEY)
    StandardEnvVar,
    /// No API key found
    NotFound,
}

impl std::fmt::Display for ApiKeySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKeySource::ConfigFile => write!(f, "config file"),
            ApiKeySource::SageEnvVar => write!(f, "SAGE_*_API_KEY env"),
            ApiKeySource::StandardEnvVar => write!(f, "env variable"),
            ApiKeySource::NotFound => write!(f, "not found"),
        }
    }
}

/// Result of API key resolution with source information
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    /// The API key value (if found)
    pub key: Option<String>,
    /// Where the key was found
    pub source: ApiKeySource,
    /// The environment variable name that was used (if from env)
    pub env_var_name: Option<String>,
}

impl ApiKeyInfo {
    /// Check if a valid API key was found
    pub fn is_valid(&self) -> bool {
        self.key.is_some()
    }

    /// Get a display-safe version (masked) of the API key
    pub fn masked_key(&self) -> Option<String> {
        self.key.as_ref().map(|k| mask_api_key(k))
    }
}

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
    /// Timeout configuration
    ///
    /// Controls connection and request timeouts. If not specified, uses default values:
    /// - Connection timeout: 30 seconds
    /// - Request timeout: 60 seconds
    #[serde(default)]
    pub timeouts: TimeoutConfig,
    /// Legacy timeout field (deprecated, use `timeouts` instead)
    ///
    /// For backward compatibility, this field is still supported.
    /// If set, it will override the request timeout in `timeouts`.
    #[serde(skip_serializing_if = "Option::is_none")]
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
            timeouts: TimeoutConfig::default(),
            timeout: None,
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

    /// Set timeout configuration
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// Set legacy timeout (deprecated, use `with_timeouts` instead)
    ///
    /// This sets only the request timeout for backward compatibility.
    #[deprecated(since = "0.1.0", note = "Use with_timeouts instead")]
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeouts.request_timeout_secs = timeout;
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
    ///
    /// Priority order:
    /// 1. SAGE_<PROVIDER>_API_KEY environment variable
    /// 2. Standard provider environment variable (e.g., ANTHROPIC_API_KEY)
    /// 3. Configuration file
    pub fn get_api_key(&self) -> Option<String> {
        self.get_api_key_info().key
    }

    /// Get detailed API key information including source
    ///
    /// Returns ApiKeyInfo with the key (if found) and where it came from.
    /// Priority order:
    /// 1. SAGE_<PROVIDER>_API_KEY environment variable
    /// 2. Standard provider environment variable (e.g., ANTHROPIC_API_KEY)
    /// 3. Configuration file
    pub fn get_api_key_info(&self) -> ApiKeyInfo {
        let provider_upper = self.name.to_uppercase();

        // 1. Try SAGE-prefixed env var first (highest priority)
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
        let standard_env_vars = get_standard_env_vars(&self.name);
        for env_var in standard_env_vars {
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
        if let Some(api_key) = &self.api_key {
            if !api_key.is_empty() {
                return ApiKeyInfo {
                    key: Some(api_key.clone()),
                    source: ApiKeySource::ConfigFile,
                    env_var_name: None,
                };
            }
        }

        // No API key found
        ApiKeyInfo {
            key: None,
            source: ApiKeySource::NotFound,
            env_var_name: None,
        }
    }

    /// Check if this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        // Ollama runs locally and doesn't need an API key
        !matches!(self.name.as_str(), "ollama")
    }

    /// Validate the API key format for this provider
    ///
    /// Returns Ok(()) if valid, Err with description if invalid.
    pub fn validate_api_key_format(&self) -> Result<(), String> {
        let key_info = self.get_api_key_info();

        if !self.requires_api_key() {
            return Ok(());
        }

        let key = match &key_info.key {
            Some(k) => k,
            None => return Err(format!(
                "API key required for '{}'. Set via {} or config file",
                self.name,
                get_standard_env_vars(&self.name).first().cloned().unwrap_or_default()
            )),
        };

        // Provider-specific validation
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
                // GLM uses JWT-like format or simple keys
                if key.len() < 10 {
                    return Err("GLM API key appears too short".to_string());
                }
            }
            _ => {
                // Generic validation: key should not be empty or placeholder
                if key.is_empty() || key.contains("your-") || key.contains("xxx") {
                    return Err("API key appears to be a placeholder".to_string());
                }
            }
        }

        Ok(())
    }

    /// Get the effective timeout configuration
    ///
    /// Handles backward compatibility with the legacy `timeout` field.
    /// If the legacy `timeout` is set, it overrides the request timeout.
    pub fn get_effective_timeouts(&self) -> TimeoutConfig {
        let mut timeouts = self.timeouts;

        // Apply legacy timeout if set (for backward compatibility)
        if let Some(legacy_timeout) = self.timeout {
            timeouts.request_timeout_secs = legacy_timeout;
        }

        timeouts
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

        // Validate timeout configuration
        let effective_timeouts = self.get_effective_timeouts();
        effective_timeouts.validate()?;

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
            .with_timeouts(TimeoutConfig::default())
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
            .with_timeouts(TimeoutConfig::default())
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
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(120_000),
                max_concurrent_requests: Some(10),
            })
    }

    /// Get default configuration for Ollama (local models)
    pub fn ollama() -> ProviderConfig {
        ProviderConfig::new("ollama")
            .with_base_url("http://localhost:11434")
            .with_timeouts(
                // Longer timeouts for local models
                TimeoutConfig::new()
                    .with_connection_timeout_secs(10)
                    .with_request_timeout_secs(120),
            )
            .with_max_retries(1)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: None, // No rate limiting for local
                tokens_per_minute: None,
                max_concurrent_requests: Some(1), // Usually one at a time for local
            })
    }

    /// Get default configuration for GLM (Zhipu AI)
    pub fn glm() -> ProviderConfig {
        ProviderConfig::new("glm")
            .with_base_url("https://open.bigmodel.cn/api/paas/v4")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(100_000),
                max_concurrent_requests: Some(10),
            })
    }

    /// Get default configuration for Azure OpenAI
    pub fn azure() -> ProviderConfig {
        ProviderConfig::new("azure")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(100_000),
                max_concurrent_requests: Some(10),
            })
    }

    /// Get default configuration for OpenRouter
    pub fn openrouter() -> ProviderConfig {
        ProviderConfig::new("openrouter")
            .with_base_url("https://openrouter.ai")
            .with_timeouts(
                // OpenRouter can be slower due to routing
                TimeoutConfig::new()
                    .with_connection_timeout_secs(30)
                    .with_request_timeout_secs(90),
            )
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(100_000),
                max_concurrent_requests: Some(10),
            })
    }

    /// Get default configuration for Doubao
    pub fn doubao() -> ProviderConfig {
        ProviderConfig::new("doubao")
            .with_timeouts(TimeoutConfig::default())
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(100_000),
                max_concurrent_requests: Some(10),
            })
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

// =============================================================================
// Helper Functions
// =============================================================================

/// Get standard environment variable names for a provider
fn get_standard_env_vars(provider: &str) -> Vec<String> {
    match provider {
        "openai" => vec!["OPENAI_API_KEY".to_string()],
        "anthropic" => vec![
            "ANTHROPIC_API_KEY".to_string(),
            "CLAUDE_API_KEY".to_string(),
        ],
        "google" => vec![
            "GOOGLE_API_KEY".to_string(),
            "GEMINI_API_KEY".to_string(),
        ],
        "azure" => vec![
            "AZURE_OPENAI_API_KEY".to_string(),
            "AZURE_API_KEY".to_string(),
        ],
        "openrouter" => vec!["OPENROUTER_API_KEY".to_string()],
        "doubao" => vec![
            "DOUBAO_API_KEY".to_string(),
            "ARK_API_KEY".to_string(),
        ],
        "glm" | "zhipu" => vec![
            "GLM_API_KEY".to_string(),
            "ZHIPU_API_KEY".to_string(),
        ],
        _ => {
            // For custom providers, try <PROVIDER>_API_KEY
            vec![format!("{}_API_KEY", provider.to_uppercase())]
        }
    }
}

/// Mask an API key for safe display
///
/// Shows first 8 and last 4 characters, masks the rest with asterisks.
fn mask_api_key(key: &str) -> String {
    let len = key.len();
    if len <= 12 {
        // Too short to mask meaningfully
        return "*".repeat(len);
    }

    let prefix = &key[..8];
    let suffix = &key[len - 4..];
    let mask_len = len - 12;

    format!("{}{}...{}", prefix, "*".repeat(mask_len.min(8)), suffix)
}

/// Display API key status for CLI
pub fn format_api_key_status(provider: &str, info: &ApiKeyInfo) -> String {
    match &info.source {
        ApiKeySource::ConfigFile => {
            format!(
                "✓ {} API key (from config): {}",
                provider,
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::SageEnvVar => {
            format!(
                "✓ {} API key (from {}): {}",
                provider,
                info.env_var_name.as_deref().unwrap_or("env"),
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::StandardEnvVar => {
            format!(
                "✓ {} API key (from {}): {}",
                provider,
                info.env_var_name.as_deref().unwrap_or("env"),
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::NotFound => {
            let env_hints = get_standard_env_vars(provider);
            format!(
                "✗ {} API key missing. Set {} or add to config",
                provider,
                env_hints.first().cloned().unwrap_or_default()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        // 26 char key: show first 8, last 4, mask middle (14-12=2, but min 8)
        assert_eq!(mask_api_key("sk-ant-api03-abc123xyz789"), "sk-ant-a********...z789");
        assert_eq!(mask_api_key("short"), "*****");
        assert_eq!(mask_api_key("exactly12ch"), "***********");
    }

    #[test]
    fn test_api_key_source_display() {
        assert_eq!(ApiKeySource::ConfigFile.to_string(), "config file");
        assert_eq!(ApiKeySource::SageEnvVar.to_string(), "SAGE_*_API_KEY env");
        assert_eq!(ApiKeySource::StandardEnvVar.to_string(), "env variable");
        assert_eq!(ApiKeySource::NotFound.to_string(), "not found");
    }

    #[test]
    fn test_get_standard_env_vars() {
        assert!(get_standard_env_vars("anthropic").contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(get_standard_env_vars("openai").contains(&"OPENAI_API_KEY".to_string()));
        assert!(get_standard_env_vars("google").contains(&"GOOGLE_API_KEY".to_string()));
        assert!(get_standard_env_vars("custom").contains(&"CUSTOM_API_KEY".to_string()));
    }

    #[test]
    fn test_provider_config_api_key_from_config() {
        let config = ProviderConfig::new("anthropic")
            .with_api_key("sk-ant-test-key-12345");

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
