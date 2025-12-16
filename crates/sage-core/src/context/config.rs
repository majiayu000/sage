//! Context management configuration

use serde::{Deserialize, Serialize};

/// Strategy for handling context overflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverflowStrategy {
    /// Simple tail truncation - remove oldest messages
    Truncate,
    /// Use LLM to summarize old messages
    Summarize,
    /// Keep first N and last M messages (sliding window)
    SlidingWindow,
    /// Summarize old messages, keep recent (hybrid approach)
    Hybrid,
}

impl Default for OverflowStrategy {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// Configuration for context window management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum context window size in tokens
    pub max_context_tokens: usize,

    /// Threshold to trigger summarization/pruning (percentage of max)
    /// e.g., 0.75 = trigger at 75% of max
    pub summarization_threshold: f32,

    /// Target size after summarization (percentage of max)
    /// e.g., 0.5 = compress to 50% of max
    pub target_size_after_summary: f32,

    /// Minimum number of recent messages to always keep
    pub min_messages_to_keep: usize,

    /// Strategy for handling overflow
    pub overflow_strategy: OverflowStrategy,

    /// Whether to preserve tool call/result messages
    pub preserve_tool_results: bool,

    /// Number of first messages to keep in sliding window mode
    pub sliding_window_first: usize,

    /// Number of last messages to keep in sliding window mode
    pub sliding_window_last: usize,

    /// Model to use for summarization (if different from main model)
    pub summarization_model: Option<String>,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 128_000,
            summarization_threshold: 0.75,
            target_size_after_summary: 0.5,
            min_messages_to_keep: 10,
            overflow_strategy: OverflowStrategy::Hybrid,
            preserve_tool_results: true,
            sliding_window_first: 3,
            sliding_window_last: 15,
            summarization_model: None,
        }
    }
}

impl ContextConfig {
    /// Create a new context config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get configuration optimized for a specific provider/model
    pub fn for_provider(provider: &str, model: &str) -> Self {
        match (provider.to_lowercase().as_str(), model) {
            // OpenAI models
            ("openai", m) if m.contains("gpt-4-turbo") || m.contains("gpt-4o") => Self {
                max_context_tokens: 128_000,
                summarization_threshold: 0.75,
                ..Default::default()
            },
            ("openai", m) if m.contains("gpt-4") => Self {
                max_context_tokens: 8_192,
                summarization_threshold: 0.70,
                min_messages_to_keep: 5,
                ..Default::default()
            },
            ("openai", m) if m.contains("gpt-3.5") => Self {
                max_context_tokens: 16_385,
                summarization_threshold: 0.70,
                min_messages_to_keep: 8,
                ..Default::default()
            },

            // Anthropic models
            ("anthropic", m) if m.contains("claude-3") || m.contains("claude-opus") => Self {
                max_context_tokens: 200_000,
                summarization_threshold: 0.80,
                min_messages_to_keep: 15,
                ..Default::default()
            },
            ("anthropic", _) => Self {
                max_context_tokens: 100_000,
                summarization_threshold: 0.75,
                ..Default::default()
            },

            // Google models
            ("google", m) if m.contains("gemini-1.5") => Self {
                max_context_tokens: 1_000_000,
                summarization_threshold: 0.90,
                min_messages_to_keep: 20,
                ..Default::default()
            },
            ("google", _) => Self {
                max_context_tokens: 32_000,
                summarization_threshold: 0.75,
                ..Default::default()
            },

            // Default
            _ => Self::default(),
        }
    }

    /// Set maximum context tokens
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_context_tokens = max;
        self
    }

    /// Set summarization threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.summarization_threshold = threshold.clamp(0.1, 0.99);
        self
    }

    /// Set overflow strategy
    pub fn with_strategy(mut self, strategy: OverflowStrategy) -> Self {
        self.overflow_strategy = strategy;
        self
    }

    /// Set minimum messages to keep
    pub fn with_min_messages(mut self, min: usize) -> Self {
        self.min_messages_to_keep = min;
        self
    }

    /// Set whether to preserve tool results
    pub fn with_preserve_tools(mut self, preserve: bool) -> Self {
        self.preserve_tool_results = preserve;
        self
    }

    /// Get the threshold in tokens
    pub fn threshold_tokens(&self) -> usize {
        (self.max_context_tokens as f32 * self.summarization_threshold) as usize
    }

    /// Get the target size in tokens
    pub fn target_tokens(&self) -> usize {
        (self.max_context_tokens as f32 * self.target_size_after_summary) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContextConfig::default();
        assert_eq!(config.max_context_tokens, 128_000);
        assert_eq!(config.summarization_threshold, 0.75);
        assert_eq!(config.overflow_strategy, OverflowStrategy::Hybrid);
    }

    #[test]
    fn test_provider_specific_config() {
        let anthropic = ContextConfig::for_provider("anthropic", "claude-3.5-sonnet");
        assert_eq!(anthropic.max_context_tokens, 200_000);
        assert_eq!(anthropic.summarization_threshold, 0.80);

        let openai = ContextConfig::for_provider("openai", "gpt-4-turbo");
        assert_eq!(openai.max_context_tokens, 128_000);

        let google = ContextConfig::for_provider("google", "gemini-1.5-pro");
        assert_eq!(google.max_context_tokens, 1_000_000);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ContextConfig::new()
            .with_max_tokens(50_000)
            .with_threshold(0.6)
            .with_strategy(OverflowStrategy::Summarize)
            .with_min_messages(5);

        assert_eq!(config.max_context_tokens, 50_000);
        assert_eq!(config.summarization_threshold, 0.6);
        assert_eq!(config.overflow_strategy, OverflowStrategy::Summarize);
        assert_eq!(config.min_messages_to_keep, 5);
    }

    #[test]
    fn test_threshold_tokens() {
        let config = ContextConfig::new()
            .with_max_tokens(100_000)
            .with_threshold(0.75);

        assert_eq!(config.threshold_tokens(), 75_000);
    }

    #[test]
    fn test_target_tokens() {
        let config = ContextConfig::default();
        let target = config.target_tokens();
        assert_eq!(target, (128_000.0 * 0.5) as usize);
    }

    #[test]
    fn test_threshold_clamping() {
        let config = ContextConfig::new().with_threshold(1.5); // Should clamp to 0.99
        assert_eq!(config.summarization_threshold, 0.99);

        let config = ContextConfig::new().with_threshold(0.0); // Should clamp to 0.1
        assert_eq!(config.summarization_threshold, 0.1);
    }
}
