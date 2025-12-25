//! Core compaction operation logic

use super::partition;
use super::result::CompactResult;
use super::stats::AutoCompactStats;
use crate::context::compact::{
    CompactOperationResult, create_compact_boundary, create_compact_summary,
};
use crate::llm::LlmMessage;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Execute a compaction operation on messages
#[allow(clippy::too_many_arguments)]
pub fn execute_compaction(
    to_keep: Vec<LlmMessage>,
    to_compact: Vec<LlmMessage>,
    summary_content: String,
    messages_before: usize,
    tokens_before: usize,
    compact_id: Uuid,
    timestamp: DateTime<Utc>,
) -> (CompactOperationResult, String) {
    let summary_preview = summary_content.chars().take(200).collect::<String>();

    // Create boundary and summary messages
    let boundary_message = create_compact_boundary(compact_id, timestamp);
    let summary_message = create_compact_summary(
        summary_content,
        compact_id,
        to_compact.len(),
        tokens_before,
        partition::estimate_tokens(&to_keep),
    );

    // Build operation result
    let operation_result = CompactOperationResult {
        compact_id,
        timestamp,
        messages_before,
        messages_after: to_keep.len() + 2, // +2 for boundary and summary
        tokens_before,
        tokens_after: partition::estimate_tokens(&to_keep)
            + partition::estimate_tokens(std::slice::from_ref(&boundary_message))
            + partition::estimate_tokens(std::slice::from_ref(&summary_message)),
        boundary_message: boundary_message.clone(),
        summary_message: summary_message.clone(),
        messages_to_keep: to_keep.clone(),
    };

    (operation_result, summary_preview)
}

/// Build new message list after compaction
pub fn build_new_messages(
    original_messages: &[LlmMessage],
    operation_result: &CompactOperationResult,
) -> Vec<LlmMessage> {
    // Build new message list: keep messages before active + new compacted messages
    let boundary_index =
        crate::context::compact::find_last_compact_boundary_index(original_messages);
    let mut new_messages = if let Some(idx) = boundary_index {
        // Keep everything before (and including) the old boundary
        original_messages[..=idx].to_vec()
    } else {
        Vec::new()
    };

    // Add new compacted messages
    new_messages.extend(operation_result.build_compacted_messages());

    new_messages
}

/// Update statistics after a successful compaction
pub fn update_stats(
    stats: &mut AutoCompactStats,
    tokens_before: usize,
    tokens_after: usize,
    messages_compacted: usize,
    timestamp: DateTime<Utc>,
    compact_id: Uuid,
) {
    stats.total_compactions += 1;
    stats.total_tokens_saved += tokens_before.saturating_sub(tokens_after) as u64;
    stats.total_messages_compacted += messages_compacted as u64;
    stats.last_compaction = Some(timestamp);
    stats.last_compact_id = Some(compact_id);
}

/// Build the final compact result
pub fn build_compact_result(
    messages_before: usize,
    messages_after: usize,
    tokens_before: usize,
    tokens_after: usize,
    messages_compacted: usize,
    timestamp: DateTime<Utc>,
    summary_preview: String,
    compact_id: Uuid,
) -> CompactResult {
    CompactResult {
        was_compacted: true,
        messages_before,
        messages_after,
        tokens_before,
        tokens_after,
        messages_compacted,
        compacted_at: Some(timestamp),
        summary_preview: Some(summary_preview),
        compact_id: Some(compact_id),
    }
}
