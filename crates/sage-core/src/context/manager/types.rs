//! Types for context manager results and statistics

use crate::llm::LlmMessage;

/// Result of preparing messages for LLM call
#[derive(Debug, Clone)]
pub struct PrepareResult {
    /// The prepared messages
    pub messages: Vec<LlmMessage>,
    /// Whether messages were pruned
    pub was_pruned: bool,
    /// Whether summarization was applied
    pub was_summarized: bool,
    /// Original token count before processing
    pub original_tokens: usize,
    /// Final token count after processing
    pub final_tokens: usize,
    /// Number of messages removed
    pub removed_count: usize,
}

impl PrepareResult {
    /// Get the token reduction
    pub fn tokens_saved(&self) -> usize {
        self.original_tokens.saturating_sub(self.final_tokens)
    }

    /// Get the compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.original_tokens == 0 {
            1.0
        } else {
            self.final_tokens as f32 / self.original_tokens as f32
        }
    }
}

/// Context usage statistics
#[derive(Debug, Clone)]
pub struct ContextUsageStats {
    /// Current token count
    pub current_tokens: usize,
    /// Maximum allowed tokens
    pub max_tokens: usize,
    /// Threshold for triggering summarization
    pub threshold_tokens: usize,
    /// Usage as percentage
    pub usage_percentage: f32,
    /// Number of messages
    pub messages_count: usize,
    /// Whether approaching the limit
    pub is_approaching_limit: bool,
    /// Whether over the limit
    pub is_over_limit: bool,
}

impl ContextUsageStats {
    /// Get remaining tokens before threshold
    pub fn tokens_until_threshold(&self) -> usize {
        self.threshold_tokens.saturating_sub(self.current_tokens)
    }

    /// Get remaining tokens before limit
    pub fn tokens_until_limit(&self) -> usize {
        self.max_tokens.saturating_sub(self.current_tokens)
    }
}
