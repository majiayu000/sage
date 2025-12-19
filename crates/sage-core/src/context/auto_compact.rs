//! Auto-Compact feature for automatic context management
//!
//! This module implements automatic context compression similar to Claude Code.
//! When the conversation context exceeds a configurable threshold (default 95%),
//! it automatically summarizes the conversation history to reduce token usage.
//!
//! ## Features
//!
//! - Automatic activation when context exceeds capacity threshold
//! - Configurable threshold (default 95% of max context)
//! - Preserves recent messages and important context
//! - Optional custom summarization instructions
//! - Manual trigger via `/compact` equivalent
//!
//! ## Usage
//!
//! ```ignore
//! let auto_compact = AutoCompact::new(config, llm_client);
//!
//! // Check and auto-compact if needed
//! let result = auto_compact.check_and_compact(&mut messages).await?;
//! if result.was_compacted {
//!     println!("Compacted {} messages, saved {} tokens", result.messages_compacted, result.tokens_saved);
//! }
//!
//! // Manual compact with custom instructions
//! auto_compact.compact_with_instructions(&mut messages, "Focus on code samples").await?;
//! ```

use crate::error::SageResult;
use crate::llm::{LLMClient, LLMMessage, MessageRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for auto-compact feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCompactConfig {
    /// Whether auto-compact is enabled
    pub enabled: bool,
    /// Threshold percentage of max context to trigger auto-compact (0.0 - 1.0)
    /// Default: 0.95 (95%)
    pub threshold_percentage: f32,
    /// Maximum context tokens (provider-specific)
    pub max_context_tokens: usize,
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
            threshold_percentage: 0.95,
            max_context_tokens: 128_000,
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
        let max_tokens = match provider.to_lowercase().as_str() {
            "anthropic" => {
                if model.contains("3.5") || model.contains("3-5") {
                    200_000
                } else {
                    100_000
                }
            }
            "openai" => {
                if model.contains("gpt-4-turbo") || model.contains("gpt-4o") {
                    128_000
                } else if model.contains("gpt-4") {
                    8_192
                } else {
                    16_385
                }
            }
            "google" => 1_000_000, // Gemini 1.5 Pro
            _ => 128_000,
        };

        Self {
            max_context_tokens: max_tokens,
            ..Default::default()
        }
    }

    /// Get the threshold token count
    pub fn threshold_tokens(&self) -> usize {
        (self.max_context_tokens as f32 * self.threshold_percentage) as usize
    }

    /// Get the target token count after compaction
    pub fn target_tokens(&self) -> usize {
        (self.max_context_tokens as f32 * self.target_after_compact) as usize
    }

    /// Set the threshold percentage
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold_percentage = threshold.clamp(0.1, 1.0);
        self
    }

    /// Enable or disable auto-compact
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Result of an auto-compact operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactResult {
    /// Whether compaction was performed
    pub was_compacted: bool,
    /// Number of messages before compaction
    pub messages_before: usize,
    /// Number of messages after compaction
    pub messages_after: usize,
    /// Tokens before compaction
    pub tokens_before: usize,
    /// Tokens after compaction
    pub tokens_after: usize,
    /// Number of messages compacted into summary
    pub messages_compacted: usize,
    /// When compaction occurred
    pub compacted_at: Option<DateTime<Utc>>,
    /// Summary that was generated (if any)
    pub summary_preview: Option<String>,
}

impl CompactResult {
    /// Create a result indicating no compaction was needed
    pub fn not_needed(messages_count: usize, tokens: usize) -> Self {
        Self {
            was_compacted: false,
            messages_before: messages_count,
            messages_after: messages_count,
            tokens_before: tokens,
            tokens_after: tokens,
            messages_compacted: 0,
            compacted_at: None,
            summary_preview: None,
        }
    }

    /// Get the number of tokens saved
    pub fn tokens_saved(&self) -> usize {
        self.tokens_before.saturating_sub(self.tokens_after)
    }

    /// Get the compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.tokens_before == 0 {
            1.0
        } else {
            self.tokens_after as f32 / self.tokens_before as f32
        }
    }
}

/// Auto-compact manager for automatic context compression
pub struct AutoCompact {
    /// Configuration
    config: AutoCompactConfig,
    /// LLM client for generating summaries
    llm_client: Option<Arc<LLMClient>>,
    /// Statistics
    stats: AutoCompactStats,
}

/// Statistics for auto-compact operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoCompactStats {
    /// Total number of auto-compactions performed
    pub total_compactions: u64,
    /// Total tokens saved across all compactions
    pub total_tokens_saved: u64,
    /// Total messages compacted
    pub total_messages_compacted: u64,
    /// Number of times compaction was skipped (not needed)
    pub skipped_count: u64,
    /// Last compaction time
    pub last_compaction: Option<DateTime<Utc>>,
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
    pub fn with_llm_client(config: AutoCompactConfig, llm_client: Arc<LLMClient>) -> Self {
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
    fn estimate_tokens(&self, messages: &[LLMMessage]) -> usize {
        messages
            .iter()
            .map(|m| {
                // Rough estimate: ~4 chars per token
                m.content.len() / 4 + 10 // +10 for role overhead
            })
            .sum()
    }

    /// Check if compaction is needed based on current token usage
    pub fn needs_compaction(&self, messages: &[LLMMessage]) -> bool {
        if !self.config.enabled {
            return false;
        }

        let current_tokens = self.estimate_tokens(messages);
        current_tokens >= self.config.threshold_tokens()
    }

    /// Get current context usage as a percentage
    pub fn get_usage_percentage(&self, messages: &[LLMMessage]) -> f32 {
        let current_tokens = self.estimate_tokens(messages);
        (current_tokens as f32 / self.config.max_context_tokens as f32) * 100.0
    }

    /// Check and auto-compact if needed
    ///
    /// This is the main entry point for automatic compaction.
    /// Call this before each LLM request to ensure context stays within limits.
    pub async fn check_and_compact(
        &mut self,
        messages: &mut Vec<LLMMessage>,
    ) -> SageResult<CompactResult> {
        let tokens_before = self.estimate_tokens(messages);
        let messages_before = messages.len();

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
        messages: &mut Vec<LLMMessage>,
        instructions: &str,
    ) -> SageResult<CompactResult> {
        self.compact_internal(messages, Some(instructions)).await
    }

    /// Force compaction regardless of current usage
    pub async fn force_compact(
        &mut self,
        messages: &mut Vec<LLMMessage>,
    ) -> SageResult<CompactResult> {
        self.compact_internal(messages, None).await
    }

    /// Internal compaction logic
    async fn compact_internal(
        &mut self,
        messages: &mut Vec<LLMMessage>,
        custom_instructions: Option<&str>,
    ) -> SageResult<CompactResult> {
        let tokens_before = self.estimate_tokens(messages);
        let messages_before = messages.len();

        if messages.is_empty() {
            return Ok(CompactResult::not_needed(0, 0));
        }

        // Separate messages to keep vs compact
        let (to_keep, to_compact) = self.partition_messages(messages);

        if to_compact.is_empty() {
            return Ok(CompactResult::not_needed(messages_before, tokens_before));
        }

        // Generate summary
        let summary = self.generate_summary(&to_compact, custom_instructions).await?;
        let summary_preview = summary.content.chars().take(200).collect::<String>();

        // Rebuild messages: summary + kept messages
        let mut new_messages = vec![summary];
        new_messages.extend(to_keep);

        let tokens_after = self.estimate_tokens(&new_messages);
        let messages_compacted = to_compact.len();

        // Update messages in place
        *messages = new_messages;

        // Update stats
        self.stats.total_compactions += 1;
        self.stats.total_tokens_saved += tokens_before.saturating_sub(tokens_after) as u64;
        self.stats.total_messages_compacted += messages_compacted as u64;
        self.stats.last_compaction = Some(Utc::now());

        let result = CompactResult {
            was_compacted: true,
            messages_before,
            messages_after: messages.len(),
            tokens_before,
            tokens_after,
            messages_compacted,
            compacted_at: Some(Utc::now()),
            summary_preview: Some(summary_preview),
        };

        tracing::info!(
            "Auto-compact complete: {} -> {} messages, {} -> {} tokens (saved {})",
            result.messages_before,
            result.messages_after,
            result.tokens_before,
            result.tokens_after,
            result.tokens_saved()
        );

        Ok(result)
    }

    /// Partition messages into those to keep and those to compact
    fn partition_messages(&self, messages: &[LLMMessage]) -> (Vec<LLMMessage>, Vec<LLMMessage>) {
        let mut to_keep = Vec::new();
        let mut to_compact = Vec::new();

        let preserve_count = self.config.preserve_recent_count;
        let total = messages.len();

        for (i, msg) in messages.iter().enumerate() {
            let is_recent = i >= total.saturating_sub(preserve_count);
            let is_system = msg.role == MessageRole::System && self.config.preserve_system_messages;
            let is_tool =
                msg.role == MessageRole::Tool && self.config.preserve_tool_messages;

            if is_recent || is_system || is_tool {
                to_keep.push(msg.clone());
            } else {
                to_compact.push(msg.clone());
            }
        }

        // Ensure we keep minimum messages
        while to_keep.len() < self.config.min_messages_to_keep && !to_compact.is_empty() {
            to_keep.insert(0, to_compact.pop().unwrap());
        }

        (to_keep, to_compact)
    }

    /// Generate a summary of messages
    async fn generate_summary(
        &self,
        messages: &[LLMMessage],
        custom_instructions: Option<&str>,
    ) -> SageResult<LLMMessage> {
        if let Some(client) = &self.llm_client {
            // Use LLM for intelligent summarization
            let prompt = self.build_summarization_prompt(messages, custom_instructions);

            let summary_request = vec![LLMMessage::user(prompt)];
            let response = client.chat(&summary_request, None).await?;

            Ok(LLMMessage::system(format!(
                "# Previous Conversation Summary\n\n{}\n\n---\n*Summarized {} messages via auto-compact*",
                response.content,
                messages.len()
            )))
        } else {
            // Fallback to simple extraction
            Ok(self.create_simple_summary(messages))
        }
    }

    /// Build the summarization prompt
    fn build_summarization_prompt(
        &self,
        messages: &[LLMMessage],
        custom_instructions: Option<&str>,
    ) -> String {
        let mut prompt = String::from(
            "Summarize the following conversation concisely, preserving:\n\
             - Key decisions and conclusions\n\
             - Important code snippets or technical details\n\
             - Action items or next steps\n\
             - Any errors or issues encountered\n\n",
        );

        if let Some(instructions) = custom_instructions {
            prompt.push_str(&format!("Additional focus: {}\n\n", instructions));
        }

        prompt.push_str("Conversation to summarize:\n\n");

        for msg in messages {
            prompt.push_str(&format!("[{}]: {}\n\n", msg.role, msg.content));
        }

        prompt.push_str("\nProvide a clear, structured summary:");

        prompt
    }

    /// Create a simple summary without LLM
    fn create_simple_summary(&self, messages: &[LLMMessage]) -> LLMMessage {
        let mut user_count = 0;
        let mut assistant_count = 0;
        let mut tool_count = 0;

        let mut key_points = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::User => {
                    user_count += 1;
                    // Extract first line as key point
                    if let Some(first_line) = msg.content.lines().next() {
                        if first_line.len() > 10 && key_points.len() < 5 {
                            key_points.push(format!("- User: {}", truncate(first_line, 100)));
                        }
                    }
                }
                MessageRole::Assistant => {
                    assistant_count += 1;
                    if let Some(first_line) = msg.content.lines().next() {
                        if first_line.len() > 10 && key_points.len() < 5 {
                            key_points.push(format!("- Assistant: {}", truncate(first_line, 100)));
                        }
                    }
                }
                MessageRole::Tool => tool_count += 1,
                _ => {}
            }
        }

        let summary = format!(
            r#"# Previous Conversation Summary

## Overview
- {} user messages
- {} assistant responses
- {} tool interactions

## Key Points
{}

---
*Simple summary of {} messages (auto-compact without LLM)*"#,
            user_count,
            assistant_count,
            tool_count,
            if key_points.is_empty() {
                "- (No key points extracted)".to_string()
            } else {
                key_points.join("\n")
            },
            messages.len()
        );

        LLMMessage::system(summary)
    }
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

impl Default for AutoCompact {
    fn default() -> Self {
        Self::new(AutoCompactConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_message(role: MessageRole, content: &str) -> LLMMessage {
        LLMMessage {
            role,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }

    fn create_test_messages(count: usize) -> Vec<LLMMessage> {
        let mut messages = vec![create_message(
            MessageRole::System,
            "You are a helpful assistant.",
        )];

        for i in 0..count {
            if i % 2 == 0 {
                messages.push(create_message(
                    MessageRole::User,
                    &format!("User message {} with some content to fill space", i),
                ));
            } else {
                messages.push(create_message(
                    MessageRole::Assistant,
                    &format!("Assistant response {} with additional content", i),
                ));
            }
        }

        messages
    }

    #[test]
    fn test_config_default() {
        let config = AutoCompactConfig::default();
        assert!(config.enabled);
        assert!((config.threshold_percentage - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_config_for_provider() {
        let config = AutoCompactConfig::for_provider("anthropic", "claude-3.5-sonnet");
        assert_eq!(config.max_context_tokens, 200_000);

        let config = AutoCompactConfig::for_provider("openai", "gpt-4-turbo");
        assert_eq!(config.max_context_tokens, 128_000);
    }

    #[test]
    fn test_needs_compaction() {
        let mut config = AutoCompactConfig::default();
        config.max_context_tokens = 100;
        config.threshold_percentage = 0.5; // 50 tokens threshold

        let auto_compact = AutoCompact::new(config);

        // Small messages - no compaction needed
        let small = vec![create_message(MessageRole::User, "Hi")];
        assert!(!auto_compact.needs_compaction(&small));

        // Large messages - compaction needed
        let large = vec![create_message(MessageRole::User, &"x".repeat(300))];
        assert!(auto_compact.needs_compaction(&large));
    }

    #[test]
    fn test_partition_messages() {
        let config = AutoCompactConfig {
            preserve_recent_count: 2,
            min_messages_to_keep: 3,
            preserve_system_messages: true,
            ..Default::default()
        };
        let auto_compact = AutoCompact::new(config);

        let messages = create_test_messages(10);
        let (to_keep, to_compact) = auto_compact.partition_messages(&messages);

        // Should keep system message + recent messages
        assert!(to_keep.len() >= 3);
        assert!(!to_compact.is_empty());

        // System message should be preserved
        assert!(to_keep.iter().any(|m| m.role == MessageRole::System));
    }

    #[tokio::test]
    async fn test_force_compact() {
        let config = AutoCompactConfig::default();
        let mut auto_compact = AutoCompact::new(config);

        let mut messages = create_test_messages(20);
        let result = auto_compact.force_compact(&mut messages).await.unwrap();

        assert!(result.was_compacted);
        assert!(result.messages_after < result.messages_before);
        assert!(result.tokens_after < result.tokens_before);
    }

    #[test]
    fn test_get_usage_percentage() {
        let mut config = AutoCompactConfig::default();
        config.max_context_tokens = 1000;
        let auto_compact = AutoCompact::new(config);

        // Create messages worth roughly 250 tokens
        let messages = vec![create_message(MessageRole::User, &"x".repeat(1000))];
        let usage = auto_compact.get_usage_percentage(&messages);

        assert!(usage > 0.0);
        assert!(usage <= 100.0);
    }

    #[test]
    fn test_compact_result_metrics() {
        let result = CompactResult {
            was_compacted: true,
            messages_before: 100,
            messages_after: 20,
            tokens_before: 50000,
            tokens_after: 10000,
            messages_compacted: 80,
            compacted_at: Some(Utc::now()),
            summary_preview: Some("Test summary...".to_string()),
        };

        assert_eq!(result.tokens_saved(), 40000);
        assert!((result.compression_ratio() - 0.2).abs() < 0.01);
    }
}
