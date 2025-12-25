//! Message partitioning logic for auto-compact

use crate::llm::{LlmMessage, MessageRole};

/// Partition messages into those to keep and those to compact
pub fn partition_messages(
    messages: &[LlmMessage],
    preserve_recent_count: usize,
    min_messages_to_keep: usize,
    preserve_system_messages: bool,
    preserve_tool_messages: bool,
) -> (Vec<LlmMessage>, Vec<LlmMessage>) {
    let mut to_keep = Vec::new();
    let mut to_compact = Vec::new();

    let total = messages.len();

    for (i, msg) in messages.iter().enumerate() {
        let is_recent = i >= total.saturating_sub(preserve_recent_count);
        let is_system = msg.role == MessageRole::System && preserve_system_messages;
        let is_tool = msg.role == MessageRole::Tool && preserve_tool_messages;

        // Also check if this is a compact boundary - always keep
        let is_boundary = crate::context::compact::is_compact_boundary(msg);

        if is_recent || is_system || is_tool || is_boundary {
            to_keep.push(msg.clone());
        } else {
            to_compact.push(msg.clone());
        }
    }

    // Ensure we keep minimum messages
    while to_keep.len() < min_messages_to_keep {
        if let Some(msg) = to_compact.pop() {
            to_keep.insert(0, msg);
        } else {
            break;
        }
    }

    (to_keep, to_compact)
}

/// Estimate token count for messages (simple estimation)
pub fn estimate_tokens(messages: &[LlmMessage]) -> usize {
    messages
        .iter()
        .map(|m| {
            // Rough estimate: ~4 chars per token
            m.content.len() / 4 + 10 // +10 for role overhead
        })
        .sum()
}
