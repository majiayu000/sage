//! Memory storage backends

use super::types::{Memory, MemoryId, MemoryQuery, MemoryScore, RelevanceScore};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Memory storage error
#[derive(Debug, Error)]
pub enum MemoryStorageError {
    #[error("Memory not found: {0}")]
    NotFound(MemoryId),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage corrupted: {0}")]
    Corrupted(String),

    #[error("Storage full")]
    StorageFull,
}

/// Memory storage trait
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// Store a memory
    async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryStorageError>;

    /// Get a memory by ID
    async fn get(&self, id: &MemoryId) -> Result<Option<Memory>, MemoryStorageError>;

    /// Update a memory
    async fn update(&self, memory: Memory) -> Result<(), MemoryStorageError>;

    /// Delete a memory
    async fn delete(&self, id: &MemoryId) -> Result<(), MemoryStorageError>;

    /// Search memories
    async fn search(&self, query: &MemoryQuery) -> Result<Vec<MemoryScore>, MemoryStorageError>;

    /// List all memories (paginated)
    async fn list(&self, offset: usize, limit: usize) -> Result<Vec<Memory>, MemoryStorageError>;

    /// Count total memories
    async fn count(&self) -> Result<usize, MemoryStorageError>;

    /// Clear all memories
    async fn clear(&self) -> Result<(), MemoryStorageError>;
}

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
        if memories.contains_key(&id) {
            memories.insert(id, memory);
            Ok(())
        } else {
            Err(MemoryStorageError::NotFound(id))
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
            b.score.total.partial_cmp(&a.score.total).unwrap_or(std::cmp::Ordering::Equal)
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
        Ok(file.memories.into_iter().map(|m| (m.id.clone(), m)).collect())
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
            b.score.total.partial_cmp(&a.score.total).unwrap_or(std::cmp::Ordering::Equal)
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

/// Check if a memory matches a query
fn matches_query(memory: &Memory, query: &MemoryQuery) -> bool {
    // Text filter - require at least partial match
    if let Some(ref text) = query.text {
        let text_lower = text.to_lowercase();
        let content_lower = memory.content.to_lowercase();

        // Check for substring match
        if !content_lower.contains(&text_lower) {
            // Check for word overlap
            let query_words: Vec<&str> = text_lower.split_whitespace().collect();
            let content_words: Vec<&str> = content_lower.split_whitespace().collect();

            let has_match = query_words.iter().any(|qw| {
                content_words.iter().any(|cw| cw.contains(qw))
            });

            if !has_match {
                return false;
            }
        }
    }

    // Type filter
    if let Some(ref mt) = query.memory_type {
        if &memory.memory_type != mt {
            return false;
        }
    }

    // Category filter
    if let Some(ref cat) = query.category {
        if &memory.category != cat {
            return false;
        }
    }

    // Tag filter
    if !query.tags.is_empty() {
        if !query.tags.iter().any(|t| memory.has_tag(t)) {
            return false;
        }
    }

    // Pinned filter
    if !query.include_pinned && memory.metadata.pinned {
        return false;
    }

    // Time filters
    if let Some(after) = query.created_after {
        if memory.metadata.created_at < after {
            return false;
        }
    }

    if let Some(after) = query.accessed_after {
        if memory.metadata.accessed_at < after {
            return false;
        }
    }

    true
}

/// Calculate content match score
fn calculate_content_score(memory: &Memory, query: &MemoryQuery) -> f32 {
    if let Some(ref text) = query.text {
        let text_lower = text.to_lowercase();
        let content_lower = memory.content.to_lowercase();

        // Check for exact match
        if content_lower.contains(&text_lower) {
            return 1.0;
        }

        // Check for word matches
        let query_words: Vec<&str> = text_lower.split_whitespace().collect();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        let mut matches = 0;
        for qw in &query_words {
            if content_words.iter().any(|cw| cw.contains(qw)) {
                matches += 1;
            }
        }

        if query_words.is_empty() {
            1.0
        } else {
            matches as f32 / query_words.len() as f32
        }
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{MemoryCategory, MemoryType};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_in_memory_store() {
        let storage = InMemoryStorage::new();

        let memory = Memory::fact("Test fact");
        let id = storage.store(memory.clone()).await.unwrap();

        let retrieved = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test fact");
    }

    #[tokio::test]
    async fn test_in_memory_update() {
        let storage = InMemoryStorage::new();

        let memory = Memory::fact("Original");
        let id = storage.store(memory).await.unwrap();

        let mut updated = storage.get(&id).await.unwrap().unwrap();
        updated.content = "Updated".to_string();
        storage.update(updated).await.unwrap();

        let retrieved = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Updated");
    }

    #[tokio::test]
    async fn test_in_memory_delete() {
        let storage = InMemoryStorage::new();

        let memory = Memory::fact("To delete");
        let id = storage.store(memory).await.unwrap();

        storage.delete(&id).await.unwrap();
        assert!(storage.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_memory_search_by_text() {
        let storage = InMemoryStorage::new();

        storage.store(Memory::fact("Rust is a systems language")).await.unwrap();
        storage.store(Memory::fact("Python is interpreted")).await.unwrap();

        let query = MemoryQuery::new().text("Rust");
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].memory.content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_in_memory_search_by_type() {
        let storage = InMemoryStorage::new();

        storage.store(Memory::fact("A fact")).await.unwrap();
        storage.store(Memory::preference("A preference")).await.unwrap();

        let query = MemoryQuery::new().memory_type(MemoryType::Fact);
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.memory_type, MemoryType::Fact);
    }

    #[tokio::test]
    async fn test_in_memory_search_by_category() {
        let storage = InMemoryStorage::new();

        storage.store(Memory::fact("Project fact").with_category(MemoryCategory::Project)).await.unwrap();
        storage.store(Memory::fact("Global fact").with_category(MemoryCategory::Global)).await.unwrap();

        let query = MemoryQuery::new().category(MemoryCategory::Project);
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.category, MemoryCategory::Project);
    }

    #[tokio::test]
    async fn test_in_memory_search_by_tag() {
        let storage = InMemoryStorage::new();

        let mut m1 = Memory::fact("Tagged");
        m1.add_tag("important");
        storage.store(m1).await.unwrap();

        storage.store(Memory::fact("Not tagged")).await.unwrap();

        let query = MemoryQuery::new().tag("important");
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].memory.has_tag("important"));
    }

    #[tokio::test]
    async fn test_in_memory_search_with_limit() {
        let storage = InMemoryStorage::new();

        for i in 0..10 {
            storage.store(Memory::fact(format!("Fact {}", i))).await.unwrap();
        }

        let query = MemoryQuery::new().limit(5);
        let results = storage.search(&query).await.unwrap();

        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_in_memory_list() {
        let storage = InMemoryStorage::new();

        for i in 0..5 {
            storage.store(Memory::fact(format!("Fact {}", i))).await.unwrap();
        }

        let all = storage.list(0, 100).await.unwrap();
        assert_eq!(all.len(), 5);

        let partial = storage.list(2, 2).await.unwrap();
        assert_eq!(partial.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_count() {
        let storage = InMemoryStorage::new();

        for i in 0..3 {
            storage.store(Memory::fact(format!("Fact {}", i))).await.unwrap();
        }

        assert_eq!(storage.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_in_memory_clear() {
        let storage = InMemoryStorage::new();

        storage.store(Memory::fact("Test")).await.unwrap();
        storage.clear().await.unwrap();

        assert_eq!(storage.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_file_storage_basic() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("memories.json");

        let storage = FileMemoryStorage::new(&path).await.unwrap();

        let memory = Memory::fact("Test fact");
        let id = storage.store(memory).await.unwrap();

        // Verify file was created
        assert!(path.exists());

        let retrieved = storage.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test fact");
    }

    #[tokio::test]
    async fn test_file_storage_persistence() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("memories.json");

        // Store memory
        {
            let storage = FileMemoryStorage::new(&path).await.unwrap();
            storage.store(Memory::fact("Persistent fact")).await.unwrap();
        }

        // Load in new instance
        {
            let storage = FileMemoryStorage::new(&path).await.unwrap();
            let all = storage.list(0, 100).await.unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].content, "Persistent fact");
        }
    }

    #[tokio::test]
    async fn test_file_storage_max_memories() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("memories.json");

        let storage = FileMemoryStorage::new(&path).await.unwrap().with_max_memories(2);

        storage.store(Memory::fact("First")).await.unwrap();
        storage.store(Memory::fact("Second")).await.unwrap();

        let result = storage.store(Memory::fact("Third")).await;
        assert!(matches!(result, Err(MemoryStorageError::StorageFull)));
    }

    #[test]
    fn test_matches_query_all() {
        let memory = Memory::fact("Test");
        let query = MemoryQuery::new();
        assert!(matches_query(&memory, &query));
    }

    #[test]
    fn test_calculate_content_score_exact() {
        let memory = Memory::fact("Rust programming language");
        let query = MemoryQuery::new().text("Rust");
        assert_eq!(calculate_content_score(&memory, &query), 1.0);
    }

    #[test]
    fn test_calculate_content_score_partial() {
        let memory = Memory::fact("Python is great");
        let query = MemoryQuery::new().text("Rust is great");
        let score = calculate_content_score(&memory, &query);
        assert!(score > 0.0 && score < 1.0);
    }
}
