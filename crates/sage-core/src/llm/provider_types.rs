//! LLM provider definitions and configurations

use serde::{Deserialize, Serialize};

/// Timeout configuration (canonical definition in `crate::types::provider`)
pub use crate::types::TimeoutConfig;

/// LLM provider enum (canonical definition in `crate::types::provider`)
pub use crate::types::LlmProvider;

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
