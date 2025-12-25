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
//! - Compact boundary markers for recovery and chaining
//! - Claude Code style 9-section summary prompt
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

use super::compact::{
    build_summary_prompt, create_compact_boundary, create_compact_summary,
    slice_from_last_compact_boundary, CompactOperationResult, SummaryPromptConfig,
};
use crate::error::SageResult;
use crate::llm::{LlmClient, LlmMessage, MessageRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

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

/// Default reserved tokens for response (matches Claude Code's value)
pub const DEFAULT_RESERVED_FOR_RESPONSE: usize = 13_000;

/// Environment variable to override auto-compact threshold percentage
pub const AUTOCOMPACT_PCT_OVERRIDE_ENV: &str = "SAGE_AUTOCOMPACT_PCT_OVERRIDE";

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
                return (self.max_context_tokens as f32 * clamped) as usize;
            }
        }

        // Default: max - reserved (Claude Code style)
        self.max_context_tokens.saturating_sub(self.reserved_for_response)
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
        (self.max_context_tokens as f32 * self.target_after_compact) as usize
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
    /// Compact operation ID (for tracking)
    pub compact_id: Option<Uuid>,
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
            compact_id: None,
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
    llm_client: Option<Arc<LlmClient>>,
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
    /// Last compact ID
    pub last_compact_id: Option<Uuid>,
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
        messages
            .iter()
            .map(|m| {
                // Rough estimate: ~4 chars per token
                m.content.len() / 4 + 10 // +10 for role overhead
            })
            .sum()
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
        let summary_preview = summary_content.chars().take(200).collect::<String>();

        // Create boundary and summary messages
        let boundary_message = create_compact_boundary(compact_id, timestamp);
        let summary_message = create_compact_summary(
            summary_content,
            compact_id,
            to_compact.len(),
            tokens_before,
            self.estimate_tokens(&to_keep),
        );

        // Build result
        let operation_result = CompactOperationResult {
            compact_id,
            timestamp,
            messages_before,
            messages_after: to_keep.len() + 2, // +2 for boundary and summary
            tokens_before,
            tokens_after: self.estimate_tokens(&to_keep)
                + self.estimate_tokens(std::slice::from_ref(&boundary_message))
                + self.estimate_tokens(std::slice::from_ref(&summary_message)),
            boundary_message: boundary_message.clone(),
            summary_message: summary_message.clone(),
            messages_to_keep: to_keep.clone(),
        };

        let tokens_after = operation_result.tokens_after;
        let messages_compacted = to_compact.len();

        // Build new message list: keep messages before active + new compacted messages
        let boundary_index = super::compact::find_last_compact_boundary_index(messages);
        let mut new_messages = if let Some(idx) = boundary_index {
            // Keep everything before (and including) the old boundary
            messages[..=idx].to_vec()
        } else {
            Vec::new()
        };

        // Add new compacted messages
        new_messages.extend(operation_result.build_compacted_messages());

        // Update messages in place
        *messages = new_messages;

        // Update stats
        self.stats.total_compactions += 1;
        self.stats.total_tokens_saved += tokens_before.saturating_sub(tokens_after) as u64;
        self.stats.total_messages_compacted += messages_compacted as u64;
        self.stats.last_compaction = Some(timestamp);
        self.stats.last_compact_id = Some(compact_id);

        let result = CompactResult {
            was_compacted: true,
            messages_before,
            messages_after: messages.len(),
            tokens_before,
            tokens_after,
            messages_compacted,
            compacted_at: Some(timestamp),
            summary_preview: Some(summary_preview),
            compact_id: Some(compact_id),
        };

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
    fn partition_messages(&self, messages: &[LlmMessage]) -> (Vec<LlmMessage>, Vec<LlmMessage>) {
        let mut to_keep = Vec::new();
        let mut to_compact = Vec::new();

        let preserve_count = self.config.preserve_recent_count;
        let total = messages.len();

        for (i, msg) in messages.iter().enumerate() {
            let is_recent = i >= total.saturating_sub(preserve_count);
            let is_system = msg.role == MessageRole::System && self.config.preserve_system_messages;
            let is_tool = msg.role == MessageRole::Tool && self.config.preserve_tool_messages;

            // Also check if this is a compact boundary - always keep
            let is_boundary = super::compact::is_compact_boundary(msg);

            if is_recent || is_system || is_tool || is_boundary {
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

    /// Generate a summary of messages using Claude Code style prompt
    async fn generate_summary(
        &self,
        messages: &[LlmMessage],
        custom_instructions: Option<&str>,
    ) -> SageResult<String> {
        if let Some(client) = &self.llm_client {
            // Use LLM for intelligent summarization with Claude Code style prompt
            let prompt_config = SummaryPromptConfig {
                custom_instructions: custom_instructions.map(|s| s.to_string()),
            };
            let prompt = build_summary_prompt(&prompt_config);

            // Format conversation for the prompt
            let conversation = self.format_messages_for_summary(messages);
            let full_prompt = format!(
                "{}\n\n---\nCONVERSATION TO SUMMARIZE:\n{}\n---",
                prompt, conversation
            );

            let summary_request = vec![LlmMessage::user(full_prompt)];
            let response = client.chat(&summary_request, None).await?;

            // Extract summary from response (handle <summary> tags if present)
            let summary = self.extract_summary(&response.content);

            Ok(format!(
                "# Previous Conversation Summary\n\n{}\n\n---\n*Summarized {} messages via auto-compact*",
                summary,
                messages.len()
            ))
        } else {
            // Fallback to simple extraction
            Ok(self.create_simple_summary(messages))
        }
    }

    /// Extract summary from LLM response, handling <summary> tags
    fn extract_summary(&self, response: &str) -> String {
        // Try to extract content between <summary> tags
        if let Some(start) = response.find("<summary>") {
            if let Some(end) = response.find("</summary>") {
                let summary_start = start + "<summary>".len();
                if summary_start < end {
                    return response[summary_start..end].trim().to_string();
                }
            }
        }
        // If no tags, return the whole response
        response.trim().to_string()
    }

    /// Format messages for the summarization prompt
    fn format_messages_for_summary(&self, messages: &[LlmMessage]) -> String {
        messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "USER",
                    MessageRole::Assistant => "ASSISTANT",
                    MessageRole::Tool => "TOOL",
                    MessageRole::System => "SYSTEM",
                };

                let content = self.truncate_content(&m.content, 1000);

                // Include tool info if present
                let tool_info = if let Some(ref tool_calls) = m.tool_calls {
                    let tools: Vec<_> = tool_calls.iter().map(|tc| tc.name.as_str()).collect();
                    format!(" [Tools: {}]", tools.join(", "))
                } else if let Some(ref tool_id) = m.tool_call_id {
                    format!(" [Response to: {}]", tool_id)
                } else {
                    String::new()
                };

                format!("[{}{}]: {}", role, tool_info, content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Truncate content to max characters
    fn truncate_content(&self, content: &str, max_chars: usize) -> String {
        if content.len() <= max_chars {
            content.to_string()
        } else {
            format!("{}...", &content[..max_chars.saturating_sub(3)])
        }
    }

    /// Create a simple summary without LLM
    fn create_simple_summary(&self, messages: &[LlmMessage]) -> String {
        let mut user_count = 0;
        let mut assistant_count = 0;
        let mut tool_count = 0;
        let mut user_messages = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::User => {
                    user_count += 1;
                    // Collect user messages (Claude Code requires all user messages)
                    if let Some(first_line) = msg.content.lines().next() {
                        if first_line.len() > 10 && user_messages.len() < 10 {
                            user_messages.push(format!(
                                "- {}",
                                self.truncate_content(first_line, 100)
                            ));
                        }
                    }
                }
                MessageRole::Assistant => assistant_count += 1,
                MessageRole::Tool => tool_count += 1,
                _ => {}
            }
        }

        format!(
            r#"# Previous Conversation Summary

## Overview
- {} user messages
- {} assistant responses
- {} tool interactions

## User Messages
{}

---
*Simple summary of {} messages (auto-compact without LLM)*"#,
            user_count,
            assistant_count,
            tool_count,
            if user_messages.is_empty() {
                "- (No significant user messages captured)".to_string()
            } else {
                user_messages.join("\n")
            },
            messages.len()
        )
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
        assert_eq!(config.reserved_for_response, DEFAULT_RESERVED_FOR_RESPONSE);
        // Default: 128K - 13K = 115K threshold (~89.8%)
        assert_eq!(config.threshold_tokens(), 128_000 - 13_000);
    }

    #[test]
    fn test_config_for_provider() {
        let config = AutoCompactConfig::for_provider("anthropic", "claude-3.5-sonnet");
        assert_eq!(config.max_context_tokens, 200_000);
        assert_eq!(config.reserved_for_response, 13_000);
        // Claude 3.5: 200K - 13K = 187K threshold (~93.5%, matches Claude Code)
        assert_eq!(config.threshold_tokens(), 187_000);

        let config = AutoCompactConfig::for_provider("openai", "gpt-4-turbo");
        assert_eq!(config.max_context_tokens, 128_000);
        assert_eq!(config.reserved_for_response, 10_000);
    }

    #[test]
    fn test_threshold_percentage() {
        let config = AutoCompactConfig::for_provider("anthropic", "claude-3.5-sonnet");
        let pct = config.threshold_percentage();
        // 187K / 200K = 0.935 (93.5%)
        assert!((pct - 0.935).abs() < 0.01);
    }

    #[test]
    fn test_needs_compaction() {
        let config = AutoCompactConfig::default()
            .with_max_tokens(100)
            .with_reserved_tokens(50); // 50 tokens threshold

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
        assert!(result.compact_id.is_some());
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
            compact_id: Some(Uuid::new_v4()),
        };

        assert_eq!(result.tokens_saved(), 40000);
        assert!((result.compression_ratio() - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_needs_compaction_respects_boundary() {
        let config = AutoCompactConfig::default()
            .with_max_tokens(100)
            .with_reserved_tokens(50); // 50 token threshold

        let auto_compact = AutoCompact::new(config);

        // Create messages with a boundary in the middle
        let old_large = create_message(MessageRole::User, &"x".repeat(500));
        let boundary = create_compact_boundary(Uuid::new_v4(), Utc::now());
        let new_small = create_message(MessageRole::User, "small");

        let messages = vec![old_large, boundary, new_small];

        // Should only consider messages after boundary, so no compaction needed
        assert!(!auto_compact.needs_compaction(&messages));
    }

    #[test]
    fn test_extract_summary() {
        let auto_compact = AutoCompact::default();

        // Test with tags
        let with_tags =
            "<analysis>thinking...</analysis>\n<summary>The actual summary</summary>\nextra";
        assert_eq!(
            auto_compact.extract_summary(with_tags),
            "The actual summary"
        );

        // Test without tags
        let without_tags = "Just a plain summary";
        assert_eq!(
            auto_compact.extract_summary(without_tags),
            "Just a plain summary"
        );
    }
}
