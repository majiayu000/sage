//! Context management configuration

use serde::{Deserialize, Serialize};

/// Default reserved tokens for response (matches Claude Code's value)
pub const DEFAULT_RESERVED_FOR_RESPONSE: usize = 13_000;

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
///
/// The summarization threshold is calculated as:
/// `max_context_tokens - reserved_for_response`
///
/// This follows Claude Code's design where a fixed number of tokens
/// is reserved for the model's response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum context window size in tokens
    pub max_context_tokens: usize,

    /// Tokens reserved for model response (like Claude Code's 13000)
    /// Summarization triggers when: current_tokens >= max_context_tokens - reserved_for_response
    pub reserved_for_response: usize,

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
            reserved_for_response: DEFAULT_RESERVED_FOR_RESPONSE,
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
                reserved_for_response: 10_000,
                ..Default::default()
            },
            ("openai", m) if m.contains("gpt-4") => Self {
                max_context_tokens: 8_192,
                reserved_for_response: 2_000,
                min_messages_to_keep: 5,
                ..Default::default()
            },
            ("openai", m) if m.contains("gpt-3.5") => Self {
                max_context_tokens: 16_385,
                reserved_for_response: 4_000,
                min_messages_to_keep: 8,
                ..Default::default()
            },

            // Anthropic models - match Claude Code's 13K reserved
            ("anthropic", m) if m.contains("claude-3") || m.contains("claude-opus") => Self {
                max_context_tokens: 200_000,
                reserved_for_response: 13_000, // Claude Code uses 13000
                min_messages_to_keep: 15,
                ..Default::default()
            },
            ("anthropic", _) => Self {
                max_context_tokens: 100_000,
                reserved_for_response: 10_000,
                ..Default::default()
            },

            // Google models
            ("google", m) if m.contains("gemini-1.5") => Self {
                max_context_tokens: 1_000_000,
                reserved_for_response: 20_000,
                min_messages_to_keep: 20,
                ..Default::default()
            },
            ("google", _) => Self {
                max_context_tokens: 32_000,
                reserved_for_response: 5_000,
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

    /// Set reserved tokens for response
    pub fn with_reserved_tokens(mut self, reserved: usize) -> Self {
        self.reserved_for_response = reserved;
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

    /// Get the threshold in tokens (max - reserved)
    ///
    /// This follows Claude Code's design: trigger summarization when
    /// current tokens >= max_context_tokens - reserved_for_response
    pub fn threshold_tokens(&self) -> usize {
        self.max_context_tokens
            .saturating_sub(self.reserved_for_response)
    }

    /// Get the threshold as a percentage (for display/logging)
    pub fn threshold_percentage(&self) -> f32 {
        if self.max_context_tokens == 0 {
            return 0.0;
        }
        self.threshold_tokens() as f32 / self.max_context_tokens as f32
    }

    /// Get the target size in tokens
    pub fn target_tokens(&self) -> usize {
        let result = self.max_context_tokens as f32 * self.target_size_after_summary;
        if result.is_finite() && result >= 0.0 {
            result as usize
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContextConfig::default();
        assert_eq!(config.max_context_tokens, 128_000);
        assert_eq!(config.reserved_for_response, DEFAULT_RESERVED_FOR_RESPONSE);
        assert_eq!(config.overflow_strategy, OverflowStrategy::Hybrid);
        // Default threshold: 128K - 13K = 115K
        assert_eq!(config.threshold_tokens(), 128_000 - 13_000);
    }

    #[test]
    fn test_provider_specific_config() {
        let anthropic = ContextConfig::for_provider("anthropic", "claude-3.5-sonnet");
        assert_eq!(anthropic.max_context_tokens, 200_000);
        assert_eq!(anthropic.reserved_for_response, 13_000);
        // Claude 3.5: 200K - 13K = 187K (~93.5%, matches Claude Code)
        assert_eq!(anthropic.threshold_tokens(), 187_000);

        let openai = ContextConfig::for_provider("openai", "gpt-4-turbo");
        assert_eq!(openai.max_context_tokens, 128_000);
        assert_eq!(openai.reserved_for_response, 10_000);

        let google = ContextConfig::for_provider("google", "gemini-1.5-pro");
        assert_eq!(google.max_context_tokens, 1_000_000);
        assert_eq!(google.reserved_for_response, 20_000);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ContextConfig::new()
            .with_max_tokens(50_000)
            .with_reserved_tokens(5_000)
            .with_strategy(OverflowStrategy::Summarize)
            .with_min_messages(5);

        assert_eq!(config.max_context_tokens, 50_000);
        assert_eq!(config.reserved_for_response, 5_000);
        assert_eq!(config.threshold_tokens(), 45_000);
        assert_eq!(config.overflow_strategy, OverflowStrategy::Summarize);
        assert_eq!(config.min_messages_to_keep, 5);
    }

    #[test]
    fn test_threshold_tokens() {
        let config = ContextConfig::new()
            .with_max_tokens(100_000)
            .with_reserved_tokens(25_000);

        assert_eq!(config.threshold_tokens(), 75_000);
    }

    #[test]
    fn test_threshold_percentage() {
        let config = ContextConfig::for_provider("anthropic", "claude-3.5-sonnet");
        let pct = config.threshold_percentage();
        // 187K / 200K = 0.935 (93.5%)
        assert!((pct - 0.935).abs() < 0.01);
    }

    #[test]
    fn test_target_tokens() {
        let config = ContextConfig::default();
        let target = config.target_tokens();
        assert_eq!(target, (128_000.0 * 0.5) as usize);
    }
}
