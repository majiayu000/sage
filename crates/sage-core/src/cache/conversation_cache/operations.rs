//! Main conversation cache operations

use super::eviction::{cleanup_expired_checkpoints, cleanup_oldest_conversations};
use super::storage::{CachedConversation, compute_prefix_hash};
use super::types::{
    CacheCheckpoint, CacheLookupResult, ConversationCacheConfig, ConversationCacheStats,
};
use crate::error::SageResult;
use crate::llm::LlmMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Conversation cache manager
///
/// Tracks cached conversation prefixes across multiple conversations
/// and provides efficient cache lookup for incremental caching.
#[derive(Debug)]
pub struct ConversationCache {
    /// Configuration
    config: ConversationCacheConfig,
    /// Cached conversations by ID
    conversations: Arc<RwLock<HashMap<String, CachedConversation>>>,
    /// Global statistics
    stats: Arc<RwLock<ConversationCacheStats>>,
}

impl ConversationCache {
    /// Create a new conversation cache
    pub fn new(config: ConversationCacheConfig) -> Self {
        Self {
            config,
            conversations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ConversationCacheStats::default())),
        }
    }

    /// Create with default configuration
    pub fn default_for_anthropic(model: &str) -> Self {
        Self::new(ConversationCacheConfig::for_anthropic(model))
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configuration
    pub fn config(&self) -> &ConversationCacheConfig {
        &self.config
    }

    /// Find the longest cached prefix for a conversation
    ///
    /// Returns the cache checkpoint if found, along with the number of
    /// messages that are cached.
    pub async fn find_cached_prefix(
        &self,
        conversation_id: &str,
        messages: &[LlmMessage],
    ) -> SageResult<Option<CacheLookupResult>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let mut conversations = self.conversations.write().await;
        let ttl_secs = self.config.cache_ttl_secs();

        if let Some(conversation) = conversations.get_mut(conversation_id) {
            if let Some(checkpoint) = conversation.find_longest_cached_prefix(messages, ttl_secs) {
                let mut stats = self.stats.write().await;
                stats.total_hits += 1;
                stats.tokens_saved += checkpoint.token_count as u64;
                // Estimate cost saved (assuming $3/1M input tokens, 90% savings)
                stats.cost_saved_usd += (checkpoint.token_count as f64 / 1_000_000.0) * 3.0 * 0.9;

                return Ok(Some(CacheLookupResult {
                    cached_message_count: checkpoint.message_count,
                    cached_token_count: checkpoint.token_count,
                    checkpoint_hash: checkpoint.prefix_hash.clone(),
                    hit_count: checkpoint.hit_count,
                }));
            }
        }

        let mut stats = self.stats.write().await;
        stats.total_misses += 1;
        Ok(None)
    }

    /// Record a new cache checkpoint
    ///
    /// Call this after a successful LLM response to cache the message prefix.
    pub async fn record_checkpoint(
        &self,
        conversation_id: &str,
        messages: &[LlmMessage],
        token_count: usize,
    ) -> SageResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check minimum token requirement
        if token_count < self.config.min_tokens_for_cache {
            tracing::debug!(
                "Skipping cache checkpoint: {} tokens < {} minimum",
                token_count,
                self.config.min_tokens_for_cache
            );
            return Ok(());
        }

        let prefix_hash = compute_prefix_hash(messages);
        let checkpoint = CacheCheckpoint::new(prefix_hash, messages.len(), token_count);

        let mut conversations = self.conversations.write().await;

        // Cleanup old conversations if at capacity
        if conversations.len() >= self.config.max_cached_conversations {
            cleanup_oldest_conversations(&mut conversations).await;
        }

        // Get or create conversation entry
        let conversation = conversations
            .entry(conversation_id.to_string())
            .or_insert_with(|| CachedConversation::new(conversation_id.to_string()));

        conversation.add_checkpoint(checkpoint, self.config.max_checkpoints_per_conversation);

        let mut stats = self.stats.write().await;
        stats.checkpoints_created += 1;

        tracing::debug!(
            "Recorded cache checkpoint for conversation {}: {} messages, {} tokens",
            conversation_id,
            messages.len(),
            token_count
        );

        Ok(())
    }

    /// Clear cache for a specific conversation
    pub async fn clear_conversation(&self, conversation_id: &str) -> SageResult<()> {
        let mut conversations = self.conversations.write().await;
        conversations.remove(conversation_id);
        Ok(())
    }

    /// Clear all cached conversations
    pub async fn clear_all(&self) -> SageResult<()> {
        let mut conversations = self.conversations.write().await;
        conversations.clear();

        let mut stats = self.stats.write().await;
        *stats = ConversationCacheStats::default();

        Ok(())
    }

    /// Get cache statistics
    pub async fn statistics(&self) -> ConversationCacheStats {
        self.stats.read().await.clone()
    }

    /// Get conversation count
    pub async fn conversation_count(&self) -> usize {
        self.conversations.read().await.len()
    }

    /// Cleanup expired checkpoints across all conversations
    pub async fn cleanup_expired(&self) -> SageResult<u64> {
        let mut conversations = self.conversations.write().await;
        let ttl_secs = self.config.cache_ttl_secs();

        let expired_count = cleanup_expired_checkpoints(&mut conversations, ttl_secs);

        if expired_count > 0 {
            let mut stats = self.stats.write().await;
            stats.checkpoints_expired += expired_count;
        }

        Ok(expired_count)
    }
}

impl Default for ConversationCache {
    fn default() -> Self {
        Self::new(ConversationCacheConfig::default())
    }
}
