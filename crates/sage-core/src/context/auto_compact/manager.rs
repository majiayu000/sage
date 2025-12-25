//! Core auto-compact manager implementation

use super::config::AutoCompactConfig;
use super::operations;
use super::partition;
use super::result::CompactResult;
use super::stats::AutoCompactStats;
use super::summary;
use crate::context::compact::slice_from_last_compact_boundary;
use crate::error::SageResult;
use crate::llm::{LlmClient, LlmMessage};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Auto-compact manager for automatic context compression
pub struct AutoCompact {
    /// Configuration
    config: AutoCompactConfig,
    /// LLM client for generating summaries
    llm_client: Option<Arc<LlmClient>>,
    /// Statistics
    stats: AutoCompactStats,
}

impl AutoCompact {
    /// Create a new auto-compact manager
    pub fn new(config: AutoCompactConfig) -> Self {
        Self {
            config,
            llm_client: None,
            stats: AutoCompactStats::default(),
        }
    }

    /// Create with an LLM client for intelligent summarization
    pub fn with_llm_client(config: AutoCompactConfig, llm_client: Arc<LlmClient>) -> Self {
        Self {
            config,
            llm_client: Some(llm_client),
            stats: AutoCompactStats::default(),
        }
    }

    /// Check if auto-compact is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configuration
    pub fn config(&self) -> &AutoCompactConfig {
        &self.config
    }

    /// Get statistics
    pub fn stats(&self) -> &AutoCompactStats {
        &self.stats
    }

    /// Estimate token count for messages (simple estimation)
    fn estimate_tokens(&self, messages: &[LlmMessage]) -> usize {
        partition::estimate_tokens(messages)
    }

    /// Check if compaction is needed based on current token usage
    ///
    /// Note: This only checks messages after the last compact boundary
    pub fn needs_compaction(&self, messages: &[LlmMessage]) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Only consider messages after the last compact boundary
        let active_messages = slice_from_last_compact_boundary(messages);
        let current_tokens = self.estimate_tokens(&active_messages);
        current_tokens >= self.config.threshold_tokens()
    }

    /// Get current context usage as a percentage
    pub fn get_usage_percentage(&self, messages: &[LlmMessage]) -> f32 {
        let active_messages = slice_from_last_compact_boundary(messages);
        let current_tokens = self.estimate_tokens(&active_messages);
        (current_tokens as f32 / self.config.max_context_tokens as f32) * 100.0
    }

    /// Check and auto-compact if needed
    ///
    /// This is the main entry point for automatic compaction.
    /// Call this before each LLM request to ensure context stays within limits.
    pub async fn check_and_compact(
        &mut self,
        messages: &mut Vec<LlmMessage>,
    ) -> SageResult<CompactResult> {
        // Only work with messages after the last boundary
        let active_messages = slice_from_last_compact_boundary(messages);
        let tokens_before = self.estimate_tokens(&active_messages);
        let messages_before = active_messages.len();

        if !self.needs_compaction(messages) {
            self.stats.skipped_count += 1;
            return Ok(CompactResult::not_needed(messages_before, tokens_before));
        }

        tracing::info!(
            "Auto-compact triggered: {} tokens ({:.1}% of max)",
            tokens_before,
            self.get_usage_percentage(messages)
        );

        self.compact_internal(messages, None).await
    }

    /// Compact with custom instructions (like `/compact Focus on code samples`)
    pub async fn compact_with_instructions(
        &mut self,
        messages: &mut Vec<LlmMessage>,
        instructions: &str,
    ) -> SageResult<CompactResult> {
        self.compact_internal(messages, Some(instructions)).await
    }

    /// Force compaction regardless of current usage
    pub async fn force_compact(
        &mut self,
        messages: &mut Vec<LlmMessage>,
    ) -> SageResult<CompactResult> {
        self.compact_internal(messages, None).await
    }

    /// Internal compaction logic
    async fn compact_internal(
        &mut self,
        messages: &mut Vec<LlmMessage>,
        custom_instructions: Option<&str>,
    ) -> SageResult<CompactResult> {
        // Work with messages after the last boundary
        let active_messages = slice_from_last_compact_boundary(messages);
        let tokens_before = self.estimate_tokens(&active_messages);
        let messages_before = active_messages.len();

        if active_messages.is_empty() {
            return Ok(CompactResult::not_needed(0, 0));
        }

        // Separate messages to keep vs compact
        let (to_keep, to_compact) = self.partition_messages(&active_messages);

        if to_compact.is_empty() {
            return Ok(CompactResult::not_needed(messages_before, tokens_before));
        }

        // Generate compact ID and timestamp
        let compact_id = Uuid::new_v4();
        let timestamp = Utc::now();

        // Generate summary using Claude Code style prompt
        let summary_content = self
            .generate_summary(&to_compact, custom_instructions)
            .await?;

        // Execute compaction operation
        let (operation_result, summary_preview) = operations::execute_compaction(
            to_keep,
            to_compact,
            summary_content,
            messages_before,
            tokens_before,
            compact_id,
            timestamp,
        );

        let tokens_after = operation_result.tokens_after;
        let messages_compacted =
            operation_result.messages_before - operation_result.messages_to_keep.len();

        // Build new message list
        let new_messages = operations::build_new_messages(messages, &operation_result);
        *messages = new_messages;

        // Update stats
        operations::update_stats(
            &mut self.stats,
            tokens_before,
            tokens_after,
            messages_compacted,
            timestamp,
            compact_id,
        );

        // Build result
        let result = operations::build_compact_result(
            messages_before,
            messages.len(),
            tokens_before,
            tokens_after,
            messages_compacted,
            timestamp,
            summary_preview,
            compact_id,
        );

        tracing::info!(
            "Auto-compact complete: {} -> {} messages, {} -> {} tokens (saved {}, ID: {})",
            result.messages_before,
            result.messages_after,
            result.tokens_before,
            result.tokens_after,
            result.tokens_saved(),
            &compact_id.to_string()[..8]
        );

        Ok(result)
    }

    /// Partition messages into those to keep and those to compact
    pub(crate) fn partition_messages(
        &self,
        messages: &[LlmMessage],
    ) -> (Vec<LlmMessage>, Vec<LlmMessage>) {
        partition::partition_messages(
            messages,
            self.config.preserve_recent_count,
            self.config.min_messages_to_keep,
            self.config.preserve_system_messages,
            self.config.preserve_tool_messages,
        )
    }

    /// Generate a summary of messages using Claude Code style prompt
    async fn generate_summary(
        &self,
        messages: &[LlmMessage],
        custom_instructions: Option<&str>,
    ) -> SageResult<String> {
        summary::generate_summary(messages, custom_instructions, self.llm_client.as_deref()).await
    }
}

impl Default for AutoCompact {
    fn default() -> Self {
        Self::new(AutoCompactConfig::default())
    }
}
