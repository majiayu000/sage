//! Result types for auto-compact operations

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
