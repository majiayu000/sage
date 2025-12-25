//! Memory maintenance operations: pruning, stats, import/export

use super::config::MemoryStats;
use super::core::MemoryManager;
use crate::memory::storage::MemoryStorageError;
use crate::memory::types::Memory;
use chrono::{Duration, Utc};

impl MemoryManager {
    /// Prune old, low-relevance memories
    pub async fn prune(&self) -> Result<usize, MemoryStorageError> {
        if !self.config.enable_decay {
            return Ok(0);
        }

        let threshold_date = Utc::now() - Duration::days(self.config.decay_threshold_days);
        let all = self.storage.list(0, self.config.max_memories).await?;

        let mut pruned = 0;
        for memory in all {
            if !memory.metadata.pinned
                && memory.relevance_score() < self.config.min_relevance_threshold
                && memory.metadata.accessed_at < threshold_date
            {
                self.delete(&memory.id).await?;
                pruned += 1;
            }
        }

        Ok(pruned)
    }

    /// Get memory statistics
    pub async fn stats(&self) -> Result<MemoryStats, MemoryStorageError> {
        let all = self.storage.list(0, self.config.max_memories).await?;
        let now = Utc::now();
        let day_ago = now - Duration::days(1);

        let mut stats = MemoryStats {
            total: all.len(),
            ..Default::default()
        };

        let mut total_relevance = 0.0;

        for memory in &all {
            // By type
            *stats
                .by_type
                .entry(memory.memory_type.name().to_string())
                .or_default() += 1;

            // By category
            *stats.by_category.entry(memory.category.name()).or_default() += 1;

            // Pinned
            if memory.metadata.pinned {
                stats.pinned += 1;
            }

            // Relevance
            total_relevance += memory.relevance_score();

            // Recent activity
            if memory.metadata.created_at > day_ago {
                stats.created_last_24h += 1;
            }
            if memory.metadata.accessed_at > day_ago {
                stats.accessed_last_24h += 1;
            }
        }

        if stats.total > 0 {
            stats.avg_relevance = total_relevance / stats.total as f32;
        }

        Ok(stats)
    }

    /// Clear all memories
    pub async fn clear(&self) -> Result<(), MemoryStorageError> {
        self.index.write().await.clear();
        self.storage.clear().await
    }

    /// Export memories to JSON
    pub async fn export(&self) -> Result<String, MemoryStorageError> {
        let all = self.storage.list(0, self.config.max_memories).await?;
        Ok(serde_json::to_string_pretty(&all)?)
    }

    /// Import memories from JSON
    pub async fn import(&self, json: &str) -> Result<usize, MemoryStorageError> {
        let memories: Vec<Memory> = serde_json::from_str(json)?;
        let count = memories.len();

        for memory in memories {
            self.store(memory).await?;
        }

        Ok(count)
    }
}
