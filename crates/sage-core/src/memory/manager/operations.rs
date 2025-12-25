//! Memory CRUD and search operations

use super::core::MemoryManager;
use super::helpers::calculate_similarity;
use crate::memory::storage::MemoryStorageError;
use crate::memory::types::{
    Memory, MemoryCategory, MemoryId, MemoryMetadata, MemoryQuery, MemoryScore, MemorySource,
    MemoryType,
};

impl MemoryManager {
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

    pub(crate) async fn find_similar(
        &self,
        memory: &Memory,
    ) -> Result<Option<Memory>, MemoryStorageError> {
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
}
