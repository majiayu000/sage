//! Result types for auto-compact operations

use crate::llm::LlmMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    /// The boundary marker message (present when compaction was performed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary_message: Option<LlmMessage>,
    /// The summary message (present when compaction was performed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_message: Option<LlmMessage>,
    /// Messages to keep after boundary (present when compaction was performed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_to_keep: Option<Vec<LlmMessage>>,
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
            boundary_message: None,
            summary_message: None,
            messages_to_keep: None,
        }
    }

    /// Get the number of tokens saved
    pub fn tokens_saved(&self) -> usize {
        self.tokens_before.saturating_sub(self.tokens_after)
    }

    /// Get the compression ratio (0.0 = full compression, 1.0 = no compression)
    pub fn compression_ratio(&self) -> f32 {
        if self.tokens_before == 0 {
            1.0
        } else {
            self.tokens_after as f32 / self.tokens_before as f32
        }
    }

    /// Build the final message list after compaction
    ///
    /// Returns None if this result has no compaction data.
    pub fn build_compacted_messages(&self) -> Option<Vec<LlmMessage>> {
        let boundary = self.boundary_message.as_ref()?;
        let summary = self.summary_message.as_ref()?;
        let to_keep = self.messages_to_keep.as_ref()?;

        let mut result = vec![boundary.clone(), summary.clone()];
        result.extend(to_keep.clone());
        Some(result)
    }
}
