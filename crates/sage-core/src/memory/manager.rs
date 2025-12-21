//! Memory manager for high-level memory operations

use super::storage::{FileMemoryStorage, InMemoryStorage, MemoryStorage, MemoryStorageError};
use super::types::{
    Memory, MemoryCategory, MemoryId, MemoryMetadata, MemoryQuery, MemoryScore, MemorySource,
    MemoryType,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory manager configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Storage path (None for in-memory)
    pub storage_path: Option<PathBuf>,
    /// Maximum memories to store
    pub max_memories: usize,
    /// Enable automatic decay
    pub enable_decay: bool,
    /// Days after which unpinned memories with low relevance are pruned
    pub decay_threshold_days: i64,
    /// Minimum relevance score to keep
    pub min_relevance_threshold: f32,
    /// Auto-save interval (0 to disable)
    pub auto_save_interval_secs: u64,
    /// Enable duplicate detection
    pub deduplicate: bool,
    /// Similarity threshold for deduplication (0.0 - 1.0)
    pub dedup_threshold: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            storage_path: None,
            max_memories: 10000,
            enable_decay: true,
            decay_threshold_days: 30,
            min_relevance_threshold: 0.1,
            auto_save_interval_secs: 0,
            deduplicate: true,
            dedup_threshold: 0.9,
        }
    }
}

impl MemoryConfig {
    /// Create config with file storage
    pub fn with_file_storage(path: impl AsRef<Path>) -> Self {
        Self {
            storage_path: Some(path.as_ref().to_path_buf()),
            ..Default::default()
        }
    }

    /// Set max memories
    pub fn max_memories(mut self, max: usize) -> Self {
        self.max_memories = max;
        self
    }

    /// Disable decay
    pub fn without_decay(mut self) -> Self {
        self.enable_decay = false;
        self
    }

    /// Set decay threshold
    pub fn decay_after_days(mut self, days: i64) -> Self {
        self.decay_threshold_days = days;
        self
    }

    /// Disable deduplication
    pub fn without_deduplication(mut self) -> Self {
        self.deduplicate = false;
        self
    }
}

/// Memory statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total memories
    pub total: usize,
    /// By type
    pub by_type: std::collections::HashMap<String, usize>,
    /// By category
    pub by_category: std::collections::HashMap<String, usize>,
    /// Pinned count
    pub pinned: usize,
    /// Average relevance score
    pub avg_relevance: f32,
    /// Memories created in last 24h
    pub created_last_24h: usize,
    /// Memories accessed in last 24h
    pub accessed_last_24h: usize,
}

/// Memory manager
pub struct MemoryManager {
    storage: Arc<dyn MemoryStorage>,
    config: MemoryConfig,
    index: Arc<RwLock<MemoryIndex>>,
}

/// Simple in-memory index for fast lookups
#[derive(Debug, Default)]
struct MemoryIndex {
    by_type: std::collections::HashMap<MemoryType, Vec<MemoryId>>,
    by_category: std::collections::HashMap<MemoryCategory, Vec<MemoryId>>,
    by_tag: std::collections::HashMap<String, Vec<MemoryId>>,
}

impl MemoryIndex {
    fn add(&mut self, memory: &Memory) {
        self.by_type
            .entry(memory.memory_type)
            .or_default()
            .push(memory.id.clone());

        self.by_category
            .entry(memory.category.clone())
            .or_default()
            .push(memory.id.clone());

        for tag in &memory.metadata.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .push(memory.id.clone());
        }
    }

    fn remove(&mut self, memory: &Memory) {
        if let Some(ids) = self.by_type.get_mut(&memory.memory_type) {
            ids.retain(|id| id != &memory.id);
        }

        if let Some(ids) = self.by_category.get_mut(&memory.category) {
            ids.retain(|id| id != &memory.id);
        }

        for tag in &memory.metadata.tags {
            if let Some(ids) = self.by_tag.get_mut(tag) {
                ids.retain(|id| id != &memory.id);
            }
        }
    }

    fn clear(&mut self) {
        self.by_type.clear();
        self.by_category.clear();
        self.by_tag.clear();
    }
}

impl MemoryManager {
    /// Create a new memory manager
    pub async fn new(config: MemoryConfig) -> Result<Self, MemoryStorageError> {
        let storage: Arc<dyn MemoryStorage> = if let Some(ref path) = config.storage_path {
            Arc::new(
                FileMemoryStorage::new(path)
                    .await?
                    .with_max_memories(config.max_memories),
            )
        } else {
            Arc::new(InMemoryStorage::new())
        };

        let manager = Self {
            storage,
            config,
            index: Arc::new(RwLock::new(MemoryIndex::default())),
        };

        // Build index from existing memories
        manager.rebuild_index().await?;

        Ok(manager)
    }

    /// Store a new memory
    pub async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryStorageError> {
        // Check for duplicates if enabled
        if self.config.deduplicate {
            if let Some(existing) = self.find_similar(&memory).await? {
                // Update existing instead of creating new
                let mut updated = existing;
                updated.metadata.touch();
                updated.metadata.access_count += 1;
                self.storage.update(updated.clone()).await?;
                return Ok(updated.id);
            }
        }

        let id = self.storage.store(memory.clone()).await?;
        self.index.write().await.add(&memory);
        Ok(id)
    }

    /// Get a memory by ID
    pub async fn get(&self, id: &MemoryId) -> Result<Option<Memory>, MemoryStorageError> {
        if let Some(mut memory) = self.storage.get(id).await? {
            memory.metadata.touch();
            self.storage.update(memory.clone()).await?;
            Ok(Some(memory))
        } else {
            Ok(None)
        }
    }

    /// Search memories
    pub async fn search(
        &self,
        query: &MemoryQuery,
    ) -> Result<Vec<MemoryScore>, MemoryStorageError> {
        self.storage.search(query).await
    }

    /// Find memories by text
    pub async fn find(&self, text: &str) -> Result<Vec<Memory>, MemoryStorageError> {
        let query = MemoryQuery::new().text(text).limit(10);
        let results = self.storage.search(&query).await?;
        Ok(results.into_iter().map(|ms| ms.memory).collect())
    }

    /// Find memories by type
    pub async fn find_by_type(
        &self,
        memory_type: MemoryType,
    ) -> Result<Vec<Memory>, MemoryStorageError> {
        let query = MemoryQuery::new().memory_type(memory_type);
        let results = self.storage.search(&query).await?;
        Ok(results.into_iter().map(|ms| ms.memory).collect())
    }

    /// Find memories by category
    pub async fn find_by_category(
        &self,
        category: MemoryCategory,
    ) -> Result<Vec<Memory>, MemoryStorageError> {
        let query = MemoryQuery::new().category(category);
        let results = self.storage.search(&query).await?;
        Ok(results.into_iter().map(|ms| ms.memory).collect())
    }

    /// Get all facts
    pub async fn facts(&self) -> Result<Vec<Memory>, MemoryStorageError> {
        self.find_by_type(MemoryType::Fact).await
    }

    /// Get all preferences
    pub async fn preferences(&self) -> Result<Vec<Memory>, MemoryStorageError> {
        self.find_by_type(MemoryType::Preference).await
    }

    /// Get all lessons
    pub async fn lessons(&self) -> Result<Vec<Memory>, MemoryStorageError> {
        self.find_by_type(MemoryType::Lesson).await
    }

    /// Remember a fact
    pub async fn remember_fact(
        &self,
        content: impl Into<String>,
        source: MemorySource,
    ) -> Result<MemoryId, MemoryStorageError> {
        let memory = Memory::fact(content).with_metadata(MemoryMetadata::with_source(source));
        self.store(memory).await
    }

    /// Remember a preference
    pub async fn remember_preference(
        &self,
        content: impl Into<String>,
    ) -> Result<MemoryId, MemoryStorageError> {
        let memory = Memory::preference(content)
            .with_metadata(MemoryMetadata::with_source(MemorySource::User).with_pinned(true));
        self.store(memory).await
    }

    /// Remember a lesson learned
    pub async fn remember_lesson(
        &self,
        content: impl Into<String>,
    ) -> Result<MemoryId, MemoryStorageError> {
        let memory =
            Memory::lesson(content).with_metadata(MemoryMetadata::with_source(MemorySource::Agent));
        self.store(memory).await
    }

    /// Delete a memory
    pub async fn delete(&self, id: &MemoryId) -> Result<(), MemoryStorageError> {
        if let Some(memory) = self.storage.get(id).await? {
            self.index.write().await.remove(&memory);
        }
        self.storage.delete(id).await
    }

    /// Pin a memory (prevent decay)
    pub async fn pin(&self, id: &MemoryId) -> Result<(), MemoryStorageError> {
        if let Some(mut memory) = self.storage.get(id).await? {
            memory.metadata.pinned = true;
            self.storage.update(memory).await?;
        }
        Ok(())
    }

    /// Unpin a memory
    pub async fn unpin(&self, id: &MemoryId) -> Result<(), MemoryStorageError> {
        if let Some(mut memory) = self.storage.get(id).await? {
            memory.metadata.pinned = false;
            self.storage.update(memory).await?;
        }
        Ok(())
    }

    /// Get pinned memories
    pub async fn pinned(&self) -> Result<Vec<Memory>, MemoryStorageError> {
        let all = self.storage.list(0, self.config.max_memories).await?;
        Ok(all.into_iter().filter(|m| m.metadata.pinned).collect())
    }

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

    /// Get relevant context for a query (useful for RAG)
    pub async fn get_relevant_context(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<String, MemoryStorageError> {
        let search = MemoryQuery::new()
            .text(query)
            .min_relevance(0.3)
            .limit(limit);

        let results = self.storage.search(&search).await?;

        let context: Vec<String> = results
            .into_iter()
            .map(|ms| format!("[{}] {}", ms.memory.memory_type.name(), ms.memory.content))
            .collect();

        Ok(context.join("\n"))
    }

    async fn find_similar(&self, memory: &Memory) -> Result<Option<Memory>, MemoryStorageError> {
        let query = MemoryQuery::new()
            .text(&memory.content)
            .memory_type(memory.memory_type)
            .limit(5);

        let results = self.storage.search(&query).await?;

        for result in results {
            let similarity = calculate_similarity(&memory.content, &result.memory.content);
            if similarity >= self.config.dedup_threshold {
                return Ok(Some(result.memory));
            }
        }

        Ok(None)
    }

    async fn rebuild_index(&self) -> Result<(), MemoryStorageError> {
        let all = self.storage.list(0, self.config.max_memories).await?;
        let mut index = self.index.write().await;
        index.clear();

        for memory in all {
            index.add(&memory);
        }

        Ok(())
    }
}

/// Calculate text similarity (simple Jaccard-like metric)
fn calculate_similarity(a: &str, b: &str) -> f32 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_words: std::collections::HashSet<&str> = a_lower.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b_lower.split_whitespace().collect();

    if a_words.is_empty() && b_words.is_empty() {
        return 1.0;
    }

    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();

    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

/// Thread-safe shared memory manager
pub type SharedMemoryManager = Arc<MemoryManager>;

/// Create a shared memory manager
pub async fn create_memory_manager(
    config: MemoryConfig,
) -> Result<SharedMemoryManager, MemoryStorageError> {
    Ok(Arc::new(MemoryManager::new(config).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config).await.unwrap();
        assert_eq!(manager.storage.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        let memory = Memory::fact("Test fact");
        let id = manager.store(memory).await.unwrap();

        let retrieved = manager.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test fact");
    }

    #[tokio::test]
    async fn test_remember_fact() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Rust uses Cargo", MemorySource::Agent)
            .await
            .unwrap();

        let facts = manager.facts().await.unwrap();
        assert_eq!(facts.len(), 1);
        assert!(facts[0].content.contains("Cargo"));
    }

    #[tokio::test]
    async fn test_remember_preference() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_preference("User prefers dark mode")
            .await
            .unwrap();

        let prefs = manager.preferences().await.unwrap();
        assert_eq!(prefs.len(), 1);
        assert!(prefs[0].metadata.pinned); // Preferences are pinned by default
    }

    #[tokio::test]
    async fn test_remember_lesson() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_lesson("Always check return values")
            .await
            .unwrap();

        let lessons = manager.lessons().await.unwrap();
        assert_eq!(lessons.len(), 1);
    }

    #[tokio::test]
    async fn test_search() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Rust is a systems language", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Python is interpreted", MemorySource::User)
            .await
            .unwrap();

        let results = manager.find("Rust").await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_find_by_type() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("A fact", MemorySource::User)
            .await
            .unwrap();
        manager.remember_preference("A preference").await.unwrap();

        let facts = manager.find_by_type(MemoryType::Fact).await.unwrap();
        assert_eq!(facts.len(), 1);

        let prefs = manager.find_by_type(MemoryType::Preference).await.unwrap();
        assert_eq!(prefs.len(), 1);
    }

    #[tokio::test]
    async fn test_pin_unpin() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        let id = manager
            .remember_fact("Test", MemorySource::User)
            .await
            .unwrap();

        manager.pin(&id).await.unwrap();
        let pinned = manager.pinned().await.unwrap();
        assert_eq!(pinned.len(), 1);

        manager.unpin(&id).await.unwrap();
        let pinned = manager.pinned().await.unwrap();
        assert_eq!(pinned.len(), 0);
    }

    #[tokio::test]
    async fn test_delete() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        let id = manager
            .remember_fact("To delete", MemorySource::User)
            .await
            .unwrap();
        manager.delete(&id).await.unwrap();

        assert!(manager.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Fact 1", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Fact 2", MemorySource::User)
            .await
            .unwrap();
        manager.remember_preference("Pref 1").await.unwrap();

        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.by_type.get("Fact"), Some(&2));
        assert_eq!(stats.by_type.get("Preference"), Some(&1));
        assert_eq!(stats.pinned, 1); // Only preference is pinned
    }

    #[tokio::test]
    async fn test_export_import() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Fact 1", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Fact 2", MemorySource::User)
            .await
            .unwrap();

        let json = manager.export().await.unwrap();
        manager.clear().await.unwrap();

        assert_eq!(manager.storage.count().await.unwrap(), 0);

        // Import requires deduplication to be off for exact count
        let config = MemoryConfig::default().without_deduplication();
        let new_manager = MemoryManager::new(config).await.unwrap();
        let count = new_manager.import(&json).await.unwrap();

        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_relevant_context() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Rust uses Cargo for builds", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Python uses pip for packages", MemorySource::User)
            .await
            .unwrap();

        let context = manager.get_relevant_context("Rust", 5).await.unwrap();
        assert!(context.contains("Cargo"));
    }

    #[tokio::test]
    async fn test_deduplication() {
        // Use a low threshold to make deduplication work with our test strings
        let config = MemoryConfig {
            dedup_threshold: 0.5, // Lower threshold to catch similar strings
            ..MemoryConfig::default()
        };
        let manager = MemoryManager::new(config).await.unwrap();

        // Store similar memories
        manager
            .remember_fact("Rust uses Cargo", MemorySource::User)
            .await
            .unwrap();
        manager
            .remember_fact("Rust uses Cargo build", MemorySource::User)
            .await
            .unwrap();

        // With deduplication, count should be 1
        assert_eq!(manager.storage.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let manager = MemoryManager::new(MemoryConfig::default()).await.unwrap();

        manager
            .remember_fact("Test", MemorySource::User)
            .await
            .unwrap();
        manager.clear().await.unwrap();

        assert_eq!(manager.storage.count().await.unwrap(), 0);
    }

    #[test]
    fn test_calculate_similarity() {
        assert_eq!(calculate_similarity("hello world", "hello world"), 1.0);
        assert!(calculate_similarity("hello world", "goodbye moon") < 0.5);
        assert!(calculate_similarity("rust cargo", "rust cargo build") > 0.5);
    }

    #[test]
    fn test_config_builder() {
        let config = MemoryConfig::default()
            .max_memories(5000)
            .without_decay()
            .without_deduplication();

        assert_eq!(config.max_memories, 5000);
        assert!(!config.enable_decay);
        assert!(!config.deduplicate);
    }

    #[tokio::test]
    async fn test_shared_manager() {
        let config = MemoryConfig::default();
        let manager = create_memory_manager(config).await.unwrap();

        manager
            .remember_fact("Shared test", MemorySource::User)
            .await
            .unwrap();
        assert_eq!(manager.storage.count().await.unwrap(), 1);
    }
}
