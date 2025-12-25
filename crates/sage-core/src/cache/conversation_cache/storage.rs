//! Storage management for cached conversations

use super::types::{CacheCheckpoint, DEFAULT_CACHE_TTL_SECS};
use crate::llm::LlmMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Cached conversation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedConversation {
    /// Conversation ID
    pub conversation_id: String,
    /// Cache checkpoints (ordered by message count)
    pub checkpoints: Vec<CacheCheckpoint>,
    /// When this conversation cache was created
    pub created_at: DateTime<Utc>,
    /// Total cache hits for this conversation
    pub total_hits: u64,
    /// Total cache misses for this conversation
    pub total_misses: u64,
}

impl CachedConversation {
    /// Create a new cached conversation
    pub fn new(conversation_id: String) -> Self {
        Self {
            conversation_id,
            checkpoints: Vec::new(),
            created_at: Utc::now(),
            total_hits: 0,
            total_misses: 0,
        }
    }

    /// Add a new checkpoint
    pub fn add_checkpoint(&mut self, checkpoint: CacheCheckpoint, max_checkpoints: usize) {
        // Remove expired checkpoints first
        self.checkpoints
            .retain(|cp| !cp.is_expired(DEFAULT_CACHE_TTL_SECS));

        // Add new checkpoint
        self.checkpoints.push(checkpoint);

        // Sort by message count (descending) for efficient lookup
        self.checkpoints
            .sort_by(|a, b| b.message_count.cmp(&a.message_count));

        // Trim to max checkpoints
        if self.checkpoints.len() > max_checkpoints {
            self.checkpoints.truncate(max_checkpoints);
        }
    }

    /// Find the longest cached prefix for the given messages
    pub fn find_longest_cached_prefix(
        &mut self,
        messages: &[LlmMessage],
        ttl_secs: i64,
    ) -> Option<&CacheCheckpoint> {
        // Compute prefix hashes for all message counts
        let prefix_hashes: HashMap<usize, String> = (1..=messages.len())
            .map(|n| (n, compute_prefix_hash(&messages[..n])))
            .collect();

        // Find the longest matching checkpoint
        for checkpoint in &mut self.checkpoints {
            if checkpoint.is_expired(ttl_secs) {
                continue;
            }

            if let Some(hash) = prefix_hashes.get(&checkpoint.message_count) {
                if hash == &checkpoint.prefix_hash {
                    checkpoint.touch();
                    self.total_hits += 1;
                    return Some(checkpoint);
                }
            }
        }

        self.total_misses += 1;
        None
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            0.0
        } else {
            self.total_hits as f64 / total as f64
        }
    }
}

/// Compute a hash for a message prefix
pub fn compute_prefix_hash(messages: &[LlmMessage]) -> String {
    let mut hasher = Sha256::new();

    for msg in messages {
        hasher.update(msg.role.to_string().as_bytes());
        hasher.update(msg.content.as_bytes());
        if let Some(name) = &msg.name {
            hasher.update(name.as_bytes());
        }
        if let Some(tool_call_id) = &msg.tool_call_id {
            hasher.update(tool_call_id.as_bytes());
        }
    }

    format!("{:x}", hasher.finalize())
}
