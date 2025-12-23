//! LLM provider definitions and configurations

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Timeout configuration for LLM requests
///
/// Provides fine-grained control over different timeout stages:
/// - **Connection timeout**: Time allowed to establish a connection
/// - **Request timeout**: Time allowed for a complete request/response cycle
///
/// # Examples
///
/// ```rust
/// use sage_core::llm::TimeoutConfig;
///
/// // Use default timeouts (30s connection, 60s request)
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
    /// Default: 60 seconds
    #[serde(default = "TimeoutConfig::default_request_timeout")]
    pub request_timeout_secs: u64,
}

impl TimeoutConfig {
    /// Default connection timeout in seconds
    const fn default_connection_timeout() -> u64 {
        30
    }

    /// Default request timeout in seconds
    const fn default_request_timeout() -> u64 {
        60
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

/// Deprecated: Use `LlmProvider` instead
#[deprecated(since = "0.2.0", note = "Use `LlmProvider` instead")]
pub type LlmProvider = LlmProvider;

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

// ProviderConfig is now defined in config::provider module

/// Model-specific parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Model name/ID
    pub model: String,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Top-k sampling (for supported models)
    pub top_k: Option<u32>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
    /// Whether to enable parallel tool calls
    pub parallel_tool_calls: Option<bool>,
    /// Frequency penalty
    pub frequency_penalty: Option<f32>,
    /// Presence penalty
    pub presence_penalty: Option<f32>,
    /// Seed for deterministic generation
    pub seed: Option<u32>,
    /// Enable prompt caching (Anthropic only)
    ///
    /// When enabled, system prompts and tools will be cached for faster
    /// subsequent requests. Cache has a 5-minute TTL that refreshes on use.
    ///
    /// Pricing:
    /// - Cache writes: 25% more than base input tokens
    /// - Cache reads: 10% of base input tokens (90% savings!)
    ///
    /// Minimum token requirements:
    /// - Claude 3.5 Sonnet & Claude Opus: 1,024 tokens
    /// - Claude Haiku: 2,048 tokens
    pub enable_prompt_caching: Option<bool>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            stop: None,
            parallel_tool_calls: Some(true),
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            enable_prompt_caching: None,
        }
    }
}

impl ModelParameters {
    /// Create new model parameters with just the model name
    pub fn new<S: Into<String>>(model: S) -> Self {
        Self {
            model: model.into(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop: None,
            parallel_tool_calls: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            enable_prompt_caching: None,
        }
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top-p
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Enable parallel tool calls
    pub fn with_parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    /// Enable or disable prompt caching (Anthropic only)
    ///
    /// When enabled, system prompts and tools will be cached.
    pub fn with_prompt_caching(mut self, enabled: bool) -> Self {
        self.enable_prompt_caching = Some(enabled);
        self
    }

    /// Check if prompt caching is enabled
    /// Defaults to true for cost savings (90% reduction on cache reads)
    pub fn is_prompt_caching_enabled(&self) -> bool {
        self.enable_prompt_caching.unwrap_or(true)
    }
}
