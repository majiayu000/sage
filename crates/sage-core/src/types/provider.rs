//! LLM provider and timeout types shared across llm, config, agent, and builder modules

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Timeout configuration for LLM requests
///
/// Provides fine-grained control over different timeout stages:
/// - **Connection timeout**: Time allowed to establish a connection
/// - **Request timeout**: Time allowed for a complete request/response cycle
///
/// Default values are set to be generous to avoid timeout issues with slow
/// models or complex requests. For comparison, Claude Code uses 10 minutes.
///
/// # Examples
///
/// ```rust
/// use sage_core::types::TimeoutConfig;
///
/// // Use default timeouts (30s connection, 300s request)
/// let config = TimeoutConfig::default();
///
/// // Custom timeouts for slow network
/// let config = TimeoutConfig::new()
///     .with_connection_timeout_secs(10)
///     .with_request_timeout_secs(120);
///
/// // Quick timeouts for fast local models
/// let config = TimeoutConfig::quick();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Connection timeout in seconds
    ///
    /// Maximum time allowed to establish a TCP connection to the API server.
    /// Default: 30 seconds
    #[serde(default = "TimeoutConfig::default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Request timeout in seconds
    ///
    /// Maximum time allowed for the complete request/response cycle,
    /// including connection establishment, sending request, and receiving response.
    /// This is the total end-to-end timeout.
    /// Default: 300 seconds (5 minutes) - generous to avoid timeout issues
    #[serde(default = "TimeoutConfig::default_request_timeout")]
    pub request_timeout_secs: u64,
}

impl TimeoutConfig {
    /// Default connection timeout in seconds
    const fn default_connection_timeout() -> u64 {
        30
    }

    /// Default request timeout in seconds (5 minutes)
    /// This is generous to avoid timeout issues with slow models or complex requests.
    /// Claude Code uses 10 minutes (600s) as default.
    const fn default_request_timeout() -> u64 {
        300
    }

    /// Create a new timeout configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a quick timeout configuration for fast local models
    ///
    /// - Connection: 5 seconds
    /// - Request: 30 seconds
    pub fn quick() -> Self {
        Self {
            connection_timeout_secs: 5,
            request_timeout_secs: 30,
        }
    }

    /// Create a relaxed timeout configuration for slow connections or large requests
    ///
    /// - Connection: 60 seconds
    /// - Request: 300 seconds (5 minutes)
    pub fn relaxed() -> Self {
        Self {
            connection_timeout_secs: 60,
            request_timeout_secs: 300,
        }
    }

    /// Set connection timeout in seconds
    pub fn with_connection_timeout_secs(mut self, secs: u64) -> Self {
        self.connection_timeout_secs = secs;
        self
    }

    /// Set request timeout in seconds
    pub fn with_request_timeout_secs(mut self, secs: u64) -> Self {
        self.request_timeout_secs = secs;
        self
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Validate timeout configuration
    ///
    /// Returns an error if:
    /// - Any timeout is zero
    /// - Request timeout is less than connection timeout
    pub fn validate(&self) -> Result<(), String> {
        if self.connection_timeout_secs == 0 {
            return Err("Connection timeout must be greater than 0".to_string());
        }
        if self.request_timeout_secs == 0 {
            return Err("Request timeout must be greater than 0".to_string());
        }
        if self.request_timeout_secs < self.connection_timeout_secs {
            return Err(
                "Request timeout must be greater than or equal to connection timeout".to_string(),
            );
        }
        Ok(())
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connection_timeout_secs: Self::default_connection_timeout(),
            request_timeout_secs: Self::default_request_timeout(),
        }
    }
}

/// Supported LLM providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// OpenAI (GPT models)
    OpenAI,
    /// Anthropic (Claude models)
    Anthropic,
    /// Google (Gemini models)
    Google,
    /// Azure OpenAI
    Azure,
    /// OpenRouter
    OpenRouter,
    /// Doubao
    Doubao,
    /// Ollama (local models)
    Ollama,
    /// GLM (Zhipu AI)
    Glm,
    /// Custom provider
    Custom(String),
}

impl LlmProvider {
    /// Get the provider name as a string
    pub fn name(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "openai",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::Google => "google",
            LlmProvider::Azure => "azure",
            LlmProvider::OpenRouter => "openrouter",
            LlmProvider::Doubao => "doubao",
            LlmProvider::Ollama => "ollama",
            LlmProvider::Glm => "glm",
            LlmProvider::Custom(name) => name,
        }
    }
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for LlmProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LlmProvider::OpenAI),
            "anthropic" => Ok(LlmProvider::Anthropic),
            "google" => Ok(LlmProvider::Google),
            "azure" => Ok(LlmProvider::Azure),
            "openrouter" => Ok(LlmProvider::OpenRouter),
            "doubao" => Ok(LlmProvider::Doubao),
            "ollama" => Ok(LlmProvider::Ollama),
            "glm" | "zhipu" => Ok(LlmProvider::Glm),
            _ => Ok(LlmProvider::Custom(s.to_string())),
        }
    }
}
