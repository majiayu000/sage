//! Cache eviction policies

use super::storage::CachedConversation;
use std::collections::HashMap;

/// Cleanup oldest conversations when at capacity
///
/// Removes conversations with lowest hit rates, breaking ties by creation time.
pub async fn cleanup_oldest_conversations(conversations: &mut HashMap<String, CachedConversation>) {
    // Remove conversations with lowest hit rates
    let mut conv_stats: Vec<_> = conversations
        .iter()
        .map(|(id, conv)| (id.clone(), conv.hit_rate(), conv.created_at))
        .collect();

    // Sort by hit rate (ascending), then by creation time (oldest first)
    conv_stats.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.2.cmp(&b.2))
    });

    // Remove the bottom 10%
    let to_remove = (conversations.len() / 10).max(1);
    for (id, _, _) in conv_stats.into_iter().take(to_remove) {
        conversations.remove(&id);
    }
}

/// Cleanup expired checkpoints across all conversations
pub fn cleanup_expired_checkpoints(
    conversations: &mut HashMap<String, CachedConversation>,
    ttl_secs: i64,
) -> u64 {
    let mut expired_count = 0u64;

    for conversation in conversations.values_mut() {
        let before_count = conversation.checkpoints.len();
        conversation
            .checkpoints
            .retain(|cp| !cp.is_expired(ttl_secs));
        expired_count += (before_count - conversation.checkpoints.len()) as u64;
    }

    // Remove empty conversations
    conversations.retain(|_, conv| !conv.checkpoints.is_empty());

    expired_count
}
