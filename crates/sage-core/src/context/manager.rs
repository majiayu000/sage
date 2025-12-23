//! Context window manager
//!
//! This module provides the main ContextManager that orchestrates token estimation,
//! message pruning, and conversation summarization to manage the LLM context window.

use crate::error::SageResult;
use crate::llm::{LlmClient, LlmMessage};
use crate::tools::types::ToolSchema;
use std::sync::Arc;

use super::config::{ContextConfig, OverflowStrategy};
use super::estimator::TokenEstimator;
use super::pruner::{MessagePruner, PruneResult};
use super::summarizer::ConversationSummarizer;

/// Context window manager for LLM conversations
///
/// Handles automatic context management including:
/// - Token estimation before sending messages
/// - Automatic pruning when approaching limits
/// - Conversation summarization for context compression
#[derive(Clone)]
pub struct ContextManager {
    /// Configuration for context management
    config: ContextConfig,
    /// Token estimator for counting tokens
    estimator: TokenEstimator,
    /// Message pruner for reducing context size
    pruner: MessagePruner,
    /// Summarizer for compressing conversation history
    summarizer: ConversationSummarizer,
}

impl ContextManager {
    /// Create a new context manager with default configuration
    pub fn new(config: ContextConfig) -> Self {
        let estimator = TokenEstimator::new();
        let pruner = MessagePruner::new(config.clone());
        let summarizer = ConversationSummarizer::new();

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Create a context manager with an LLM client for summarization
    pub fn with_llm_client(config: ContextConfig, llm_client: Arc<LlmClient>) -> Self {
        let estimator = TokenEstimator::new();
        let pruner = MessagePruner::new(config.clone());
        let summarizer = ConversationSummarizer::with_client(llm_client);

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Create a context manager optimized for a specific provider
    pub fn for_provider(provider: &str, model: &str) -> Self {
        let config = ContextConfig::for_provider(provider, model);
        let estimator = TokenEstimator::for_provider(provider);
        let pruner = MessagePruner::new(config.clone());
        let summarizer = ConversationSummarizer::new();

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Create with custom components
    pub fn with_components(
        config: ContextConfig,
        estimator: TokenEstimator,
        summarizer: ConversationSummarizer,
    ) -> Self {
        let pruner = MessagePruner::new(config.clone());

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &ContextConfig {
        &self.config
    }

    /// Get the token estimator
    pub fn estimator(&self) -> &TokenEstimator {
        &self.estimator
    }

    /// Estimate tokens for a conversation
    pub fn estimate_tokens(&self, messages: &[LlmMessage]) -> usize {
        self.estimator.estimate_conversation(messages)
    }

    /// Estimate total tokens including tools
    pub fn estimate_request_tokens(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> usize {
        self.estimator.estimate_request(messages, tools)
    }

    /// Check if context is approaching the limit
    pub fn is_approaching_limit(&self, messages: &[LlmMessage]) -> bool {
        let current_tokens = self.estimator.estimate_conversation(messages);
        current_tokens >= self.config.threshold_tokens()
    }

    /// Check if context exceeds the maximum
    pub fn exceeds_limit(&self, messages: &[LlmMessage]) -> bool {
        let current_tokens = self.estimator.estimate_conversation(messages);
        current_tokens >= self.config.max_context_tokens
    }

    /// Get context usage statistics
    pub fn get_usage_stats(&self, messages: &[LlmMessage]) -> ContextUsageStats {
        let current_tokens = self.estimator.estimate_conversation(messages);
        let max_tokens = self.config.max_context_tokens;
        let threshold_tokens = self.config.threshold_tokens();

        ContextUsageStats {
            current_tokens,
            max_tokens,
            threshold_tokens,
            usage_percentage: (current_tokens as f32 / max_tokens as f32) * 100.0,
            messages_count: messages.len(),
            is_approaching_limit: current_tokens >= threshold_tokens,
            is_over_limit: current_tokens >= max_tokens,
        }
    }

    /// Prepare messages for an LLM call, managing context window automatically
    ///
    /// This method:
    /// 1. Estimates current token usage
    /// 2. If under threshold, returns messages unchanged
    /// 3. If over threshold, applies the configured overflow strategy
    pub async fn prepare_messages(
        &self,
        messages: Vec<LlmMessage>,
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<PrepareResult> {
        // Estimate current usage
        let total_tokens = self.estimator.estimate_request(&messages, tools);
        let threshold = self.config.threshold_tokens();

        // If under threshold, return as-is
        if total_tokens < threshold {
            return Ok(PrepareResult {
                messages,
                was_pruned: false,
                was_summarized: false,
                original_tokens: total_tokens,
                final_tokens: total_tokens,
                removed_count: 0,
            });
        }

        // Log warning about approaching limit
        tracing::warn!(
            "Context approaching limit: {}/{} tokens ({}%)",
            total_tokens,
            self.config.max_context_tokens,
            (total_tokens as f32 / self.config.max_context_tokens as f32 * 100.0) as u32
        );

        // Apply overflow strategy
        let target_tokens = self.config.target_tokens();

        match self.config.overflow_strategy {
            OverflowStrategy::Truncate | OverflowStrategy::SlidingWindow => {
                let prune_result = self.pruner.prune(messages, target_tokens);
                let kept_tokens = prune_result.kept_tokens;
                let removed_count = prune_result.removed_count();
                Ok(PrepareResult {
                    messages: prune_result.kept,
                    was_pruned: true,
                    was_summarized: false,
                    original_tokens: total_tokens,
                    final_tokens: kept_tokens,
                    removed_count,
                })
            }
            OverflowStrategy::Summarize => {
                self.summarize_and_compress(messages, total_tokens, target_tokens)
                    .await
            }
            OverflowStrategy::Hybrid => {
                self.hybrid_compress(messages, total_tokens, target_tokens)
                    .await
            }
        }
    }

    /// Summarize old messages and compress context
    async fn summarize_and_compress(
        &self,
        messages: Vec<LlmMessage>,
        original_tokens: usize,
        target_tokens: usize,
    ) -> SageResult<PrepareResult> {
        // First, prune to separate old and recent messages
        let prune_result = self.pruner.prune(messages, target_tokens);
        let kept_tokens = prune_result.kept_tokens;
        let removed_count = prune_result.removed_count();

        if prune_result.removed.is_empty() {
            // Nothing to summarize
            return Ok(PrepareResult {
                messages: prune_result.kept,
                was_pruned: true,
                was_summarized: false,
                original_tokens,
                final_tokens: kept_tokens,
                removed_count: 0,
            });
        }

        // Summarize removed messages
        let summary = self.summarizer.summarize(&prune_result.removed).await?;

        // Build new message list: summary + kept messages
        let mut new_messages = vec![summary];
        new_messages.extend(prune_result.kept);

        let final_tokens = self.estimator.estimate_conversation(&new_messages);

        Ok(PrepareResult {
            messages: new_messages,
            was_pruned: true,
            was_summarized: true,
            original_tokens,
            final_tokens,
            removed_count,
        })
    }

    /// Hybrid approach: summarize if beneficial, otherwise just prune
    async fn hybrid_compress(
        &self,
        messages: Vec<LlmMessage>,
        original_tokens: usize,
        target_tokens: usize,
    ) -> SageResult<PrepareResult> {
        // Prune first
        let prune_result = self.pruner.prune(messages, target_tokens);
        let kept_tokens = prune_result.kept_tokens;
        let removed_count = prune_result.removed_count();

        // If still over target and we have removed messages, try summarizing
        if kept_tokens > target_tokens && !prune_result.removed.is_empty() {
            let summary = self.summarizer.summarize(&prune_result.removed).await?;

            let mut new_messages = vec![summary];
            new_messages.extend(prune_result.kept);

            let final_tokens = self.estimator.estimate_conversation(&new_messages);

            // Only use summary if it actually helps
            if final_tokens < kept_tokens {
                return Ok(PrepareResult {
                    messages: new_messages,
                    was_pruned: true,
                    was_summarized: true,
                    original_tokens,
                    final_tokens,
                    removed_count,
                });
            }

            // Summary didn't help, return the original kept messages
            // But we already moved them into new_messages, so extract them back
            // Skip the first message which is the summary
            let kept_messages: Vec<_> = new_messages.into_iter().skip(1).collect();
            return Ok(PrepareResult {
                messages: kept_messages,
                was_pruned: true,
                was_summarized: false,
                original_tokens,
                final_tokens: kept_tokens,
                removed_count,
            });
        }

        // Fall back to just pruning
        Ok(PrepareResult {
            messages: prune_result.kept,
            was_pruned: true,
            was_summarized: false,
            original_tokens,
            final_tokens: kept_tokens,
            removed_count,
        })
    }

    /// Force summarization of messages (useful for manual context management)
    pub async fn force_summarize(&self, messages: &[LlmMessage]) -> SageResult<LlmMessage> {
        self.summarizer.summarize(messages).await
    }

    /// Prune messages without summarization
    pub fn prune(&self, messages: Vec<LlmMessage>, target_tokens: usize) -> PruneResult {
        self.pruner.prune(messages, target_tokens)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;
    use std::collections::HashMap;

    fn create_message(role: MessageRole, content: &str) -> LlmMessage {
        LlmMessage {
            role,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }

    fn create_test_messages(count: usize) -> Vec<LlmMessage> {
        let mut messages = vec![create_message(
            MessageRole::System,
            "You are a helpful assistant.",
        )];

        for i in 0..count {
            if i % 2 == 0 {
                messages.push(create_message(
                    MessageRole::User,
                    &format!("User message number {} with some content to fill tokens", i),
                ));
            } else {
                messages.push(create_message(
                    MessageRole::Assistant,
                    &format!(
                        "Assistant response number {} with additional content for testing",
                        i
                    ),
                ));
            }
        }

        messages
    }

    #[test]
    fn test_create_manager() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        assert_eq!(manager.config().max_context_tokens, 128_000);
    }

    #[test]
    fn test_for_provider() {
        let manager = ContextManager::for_provider("anthropic", "claude-3.5-sonnet");
        assert_eq!(manager.config().max_context_tokens, 200_000);

        let manager = ContextManager::for_provider("openai", "gpt-4-turbo");
        assert_eq!(manager.config().max_context_tokens, 128_000);
    }

    #[test]
    fn test_estimate_tokens() {
        let manager = ContextManager::new(ContextConfig::default());
        let messages = create_test_messages(5);

        let tokens = manager.estimate_tokens(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_is_approaching_limit() {
        // Create a config with very low limit for testing
        let config = ContextConfig::new()
            .with_max_tokens(100)
            .with_threshold(0.5); // 50 tokens threshold

        let manager = ContextManager::new(config);

        // Small message should be under threshold
        let small = vec![create_message(MessageRole::User, "Hi")];
        assert!(!manager.is_approaching_limit(&small));

        // Large message should be over threshold
        let large = vec![create_message(
            MessageRole::User,
            &"x".repeat(500), // ~125 tokens
        )];
        assert!(manager.is_approaching_limit(&large));
    }

    #[test]
    fn test_get_usage_stats() {
        let config = ContextConfig::new().with_max_tokens(1000);
        let manager = ContextManager::new(config);

        let messages = create_test_messages(5);
        let stats = manager.get_usage_stats(&messages);

        assert!(stats.current_tokens > 0);
        assert_eq!(stats.max_tokens, 1000);
        assert_eq!(stats.messages_count, 6); // 5 + 1 system
        assert!(stats.usage_percentage >= 0.0);
    }

    #[tokio::test]
    async fn test_prepare_messages_under_threshold() {
        let config = ContextConfig::new().with_max_tokens(100_000);
        let manager = ContextManager::new(config);

        let messages = create_test_messages(5);
        let result = manager
            .prepare_messages(messages.clone(), None)
            .await
            .unwrap();

        // Should return unchanged when under threshold
        assert!(!result.was_pruned);
        assert!(!result.was_summarized);
        assert_eq!(result.messages.len(), messages.len());
    }

    #[tokio::test]
    async fn test_prepare_messages_over_threshold_truncate() {
        let config = ContextConfig::new()
            .with_max_tokens(100)
            .with_threshold(0.5)
            .with_strategy(OverflowStrategy::Truncate)
            .with_min_messages(2);

        let manager = ContextManager::new(config);
        let messages = create_test_messages(10);

        let result = manager.prepare_messages(messages, None).await.unwrap();

        // Should be pruned
        assert!(result.was_pruned);
    }

    #[tokio::test]
    async fn test_prepare_messages_sliding_window() {
        let mut config = ContextConfig::new()
            .with_max_tokens(100)
            .with_threshold(0.5)
            .with_strategy(OverflowStrategy::SlidingWindow);
        config.sliding_window_first = 2;
        config.sliding_window_last = 2;

        let manager = ContextManager::new(config);
        let messages = create_test_messages(10);

        let result = manager.prepare_messages(messages, None).await.unwrap();

        assert!(result.was_pruned);
    }

    #[test]
    fn test_prune_direct() {
        let config = ContextConfig::new().with_min_messages(3);
        let manager = ContextManager::new(config);

        let messages = create_test_messages(10);
        let result = manager.prune(messages, 50);

        assert!(result.kept.len() >= 4); // At least 3 + system
    }

    #[test]
    fn test_prepare_result_tokens_saved() {
        let result = PrepareResult {
            messages: vec![],
            was_pruned: true,
            was_summarized: false,
            original_tokens: 1000,
            final_tokens: 600,
            removed_count: 5,
        };

        assert_eq!(result.tokens_saved(), 400);
        assert!((result.compression_ratio() - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_usage_stats_calculations() {
        let stats = ContextUsageStats {
            current_tokens: 5000,
            max_tokens: 10000,
            threshold_tokens: 7500,
            usage_percentage: 50.0,
            messages_count: 20,
            is_approaching_limit: false,
            is_over_limit: false,
        };

        assert_eq!(stats.tokens_until_threshold(), 2500);
        assert_eq!(stats.tokens_until_limit(), 5000);
    }
}
