//! In-memory storage implementation

use super::super::types::{Memory, MemoryId, MemoryQuery, MemoryScore, RelevanceScore};
use super::error::MemoryStorageError;
use super::query::{calculate_content_score, matches_query};
use super::r#trait::MemoryStorage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory storage (for testing)
#[derive(Debug)]
pub struct InMemoryStorage {
    memories: Arc<RwLock<HashMap<MemoryId, Memory>>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            memories: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStorage for InMemoryStorage {
    async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryStorageError> {
        let id = memory.id.clone();
        self.memories.write().await.insert(id.clone(), memory);
        Ok(id)
    }

    async fn get(&self, id: &MemoryId) -> Result<Option<Memory>, MemoryStorageError> {
        Ok(self.memories.read().await.get(id).cloned())
    }

    async fn update(&self, memory: Memory) -> Result<(), MemoryStorageError> {
        let id = memory.id.clone();
        let mut memories = self.memories.write().await;
        use std::collections::hash_map::Entry;
        match memories.entry(id.clone()) {
            Entry::Occupied(mut e) => {
                e.insert(memory);
                Ok(())
            }
            Entry::Vacant(_) => Err(MemoryStorageError::NotFound(id)),
        }
    }

    async fn delete(&self, id: &MemoryId) -> Result<(), MemoryStorageError> {
        self.memories.write().await.remove(id);
        Ok(())
    }

    async fn search(&self, query: &MemoryQuery) -> Result<Vec<MemoryScore>, MemoryStorageError> {
        let memories = self.memories.read().await;
        let mut results: Vec<MemoryScore> = memories
            .values()
            .filter(|m| matches_query(m, query))
            .map(|m| {
                let content_score = calculate_content_score(m, query);
                let decay_score = m.relevance_score();
                MemoryScore {
                    memory: m.clone(),
                    score: RelevanceScore {
                        content_score,
                        decay_score,
                        total: content_score * decay_score,
                    },
                }
            })
            .filter(|ms| {
                query
                    .min_relevance
                    .map(|min| ms.score.total >= min)
                    .unwrap_or(true)
            })
            .collect();

        // Sort by score
        results.sort_by(|a, b| {
            b.score
                .total
                .partial_cmp(&a.score.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn list(&self, offset: usize, limit: usize) -> Result<Vec<Memory>, MemoryStorageError> {
        let memories = self.memories.read().await;
        let mut all: Vec<Memory> = memories.values().cloned().collect();
        all.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));
        Ok(all.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self) -> Result<usize, MemoryStorageError> {
        Ok(self.memories.read().await.len())
    }

    async fn clear(&self) -> Result<(), MemoryStorageError> {
        self.memories.write().await.clear();
        Ok(())
    }
}
