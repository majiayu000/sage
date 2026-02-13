//! Configuration for auto-compact feature

use serde::{Deserialize, Serialize};

/// Default reserved tokens for response (matches Claude Code's value)
pub const DEFAULT_RESERVED_FOR_RESPONSE: usize = 13_000;

/// Environment variable to override auto-compact threshold percentage
pub const AUTOCOMPACT_PCT_OVERRIDE_ENV: &str = "SAGE_AUTOCOMPACT_PCT_OVERRIDE";

/// Configuration for auto-compact feature
///
/// The auto-compact threshold is calculated as:
/// `max_context_tokens - reserved_for_response`
///
/// This follows Claude Code's design where a fixed number of tokens
/// is reserved for the model's response, rather than using a simple percentage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCompactConfig {
    /// Whether auto-compact is enabled
    pub enabled: bool,
    /// Maximum context tokens (provider-specific)
    pub max_context_tokens: usize,
    /// Tokens reserved for model response (like Claude Code's 13000)
    /// Auto-compact triggers when: current_tokens >= max_context_tokens - reserved_for_response
    pub reserved_for_response: usize,
    /// Minimum messages to keep after compaction
    pub min_messages_to_keep: usize,
    /// Number of recent messages to always preserve
    pub preserve_recent_count: usize,
    /// Whether to preserve system messages
    pub preserve_system_messages: bool,
    /// Whether to preserve tool-related messages
    pub preserve_tool_messages: bool,
    /// Target token count after compaction (percentage of max)
    pub target_after_compact: f32,
}

impl Default for AutoCompactConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_context_tokens: 128_000,
            reserved_for_response: DEFAULT_RESERVED_FOR_RESPONSE,
            min_messages_to_keep: 10,
            preserve_recent_count: 5,
            preserve_system_messages: true,
            preserve_tool_messages: true,
            target_after_compact: 0.5, // Target 50% of max after compaction
        }
    }
}

impl AutoCompactConfig {
    /// Create config for a specific provider
    pub fn for_provider(provider: &str, model: &str) -> Self {
        let (max_tokens, reserved) = match provider.to_lowercase().as_str() {
            "anthropic" => {
                if model.contains("3.5") || model.contains("3-5") {
                    (200_000, 13_000) // Claude 3.5: 200K context, 13K reserved (like Claude Code)
                } else {
                    (100_000, 10_000)
                }
            }
            "openai" => {
                if model.contains("gpt-4-turbo") || model.contains("gpt-4o") {
                    (128_000, 10_000)
                } else if model.contains("gpt-4") {
                    (8_192, 2_000)
                } else {
                    (16_385, 4_000)
                }
            }
            "google" => (1_000_000, 20_000), // Gemini 1.5 Pro: larger context, more reserved
            _ => (128_000, DEFAULT_RESERVED_FOR_RESPONSE),
        };

        Self {
            max_context_tokens: max_tokens,
            reserved_for_response: reserved,
            ..Default::default()
        }
    }

    /// Get the threshold token count (max - reserved)
    ///
    /// This follows Claude Code's design: trigger compaction when
    /// current tokens >= max_context_tokens - reserved_for_response
    ///
    /// Supports override via SAGE_AUTOCOMPACT_PCT_OVERRIDE environment variable
    pub fn threshold_tokens(&self) -> usize {
        // Check for environment variable override
        if let Ok(pct_str) = std::env::var(AUTOCOMPACT_PCT_OVERRIDE_ENV) {
            if let Ok(pct) = pct_str.parse::<f32>() {
                let clamped = pct.clamp(0.1, 1.0);
                let result = self.max_context_tokens as f32 * clamped;
                return if result.is_finite() && result >= 0.0 {
                    result as usize
                } else {
                    0
                };
            }
        }

        // Default: max - reserved (Claude Code style)
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

    /// Get the target token count after compaction
    pub fn target_tokens(&self) -> usize {
        let result = self.max_context_tokens as f32 * self.target_after_compact;
        if result.is_finite() && result >= 0.0 {
            result as usize
        } else {
            0
        }
    }

    /// Set the reserved tokens for response
    pub fn with_reserved_tokens(mut self, reserved: usize) -> Self {
        self.reserved_for_response = reserved;
        self
    }

    /// Enable or disable auto-compact
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set max context tokens
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_context_tokens = max;
        self
    }
}
