//! Context manager operations for message preparation and compression

use crate::error::SageResult;
use crate::llm::LlmMessage;
use crate::tools::types::ToolSchema;

use super::super::config::OverflowStrategy;
use super::core::ContextManager;
use super::types::PrepareResult;

impl ContextManager {
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
        let pct = total_tokens as f64 / self.config.max_context_tokens as f64 * 100.0;
        let pct_display = if pct.is_finite() && pct >= 0.0 { (pct as u32).min(100) } else { 0 };
        tracing::warn!(
            "Context approaching limit: {}/{} tokens ({}%)",
            total_tokens,
            self.config.max_context_tokens,
            pct_display
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
    pub(super) async fn summarize_and_compress(
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
    pub(super) async fn hybrid_compress(
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
}
