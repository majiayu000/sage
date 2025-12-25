//! File-based storage implementation

use super::super::types::{Memory, MemoryId, MemoryQuery, MemoryScore, RelevanceScore};
use super::error::MemoryStorageError;
use super::query::{calculate_content_score, matches_query};
use super::r#trait::MemoryStorage;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// File-based storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryFile {
    version: u32,
    memories: Vec<Memory>,
}

impl Default for MemoryFile {
    fn default() -> Self {
        Self {
            version: 1,
            memories: Vec::new(),
        }
    }
}

/// File-based memory storage
#[derive(Debug)]
pub struct FileMemoryStorage {
    path: PathBuf,
    memories: Arc<RwLock<HashMap<MemoryId, Memory>>>,
    max_memories: usize,
}

impl FileMemoryStorage {
    /// Create a new file storage
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, MemoryStorageError> {
        let path = path.as_ref().to_path_buf();
        let memories = if path.exists() {
            Self::load_from_file(&path).await?
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            memories: Arc::new(RwLock::new(memories)),
            max_memories: 10000,
        })
    }

    /// Set maximum memories
    pub fn with_max_memories(mut self, max: usize) -> Self {
        self.max_memories = max;
        self
    }

    async fn load_from_file(path: &Path) -> Result<HashMap<MemoryId, Memory>, MemoryStorageError> {
        let content = tokio::fs::read_to_string(path).await?;
        let file: MemoryFile = serde_json::from_str(&content)?;
        Ok(file
            .memories
            .into_iter()
            .map(|m| (m.id.clone(), m))
            .collect())
    }

    async fn save_to_file(&self) -> Result<(), MemoryStorageError> {
        let memories = self.memories.read().await;
        let file = MemoryFile {
            version: 1,
            memories: memories.values().cloned().collect(),
        };

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&file)?;
        tokio::fs::write(&self.path, content).await?;
        Ok(())
    }

    /// Force save to disk
    pub async fn flush(&self) -> Result<(), MemoryStorageError> {
        self.save_to_file().await
    }

    /// Get storage path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait]
impl MemoryStorage for FileMemoryStorage {
    async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryStorageError> {
        {
            let memories = self.memories.read().await;
            if memories.len() >= self.max_memories {
                return Err(MemoryStorageError::StorageFull);
            }
        }

        let id = memory.id.clone();
        self.memories.write().await.insert(id.clone(), memory);
        self.save_to_file().await?;
        Ok(id)
    }

    async fn get(&self, id: &MemoryId) -> Result<Option<Memory>, MemoryStorageError> {
        Ok(self.memories.read().await.get(id).cloned())
    }

    async fn update(&self, memory: Memory) -> Result<(), MemoryStorageError> {
        let id = memory.id.clone();
        {
            let mut memories = self.memories.write().await;
            if !memories.contains_key(&id) {
                return Err(MemoryStorageError::NotFound(id));
            }
            memories.insert(id, memory);
        }
        self.save_to_file().await
    }

    async fn delete(&self, id: &MemoryId) -> Result<(), MemoryStorageError> {
        self.memories.write().await.remove(id);
        self.save_to_file().await
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

        results.sort_by(|a, b| {
            b.score
                .total
                .partial_cmp(&a.score.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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
        self.save_to_file().await
    }
}
