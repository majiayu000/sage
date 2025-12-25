//! Core memory manager implementation

use super::config::MemoryConfig;
use crate::memory::storage::{
    FileMemoryStorage, InMemoryStorage, MemoryStorage, MemoryStorageError,
};
use crate::memory::types::{Memory, MemoryCategory, MemoryId, MemoryType};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simple in-memory index for fast lookups
#[derive(Debug, Default)]
pub(crate) struct MemoryIndex {
    pub(crate) by_type: std::collections::HashMap<MemoryType, Vec<MemoryId>>,
    pub(crate) by_category: std::collections::HashMap<MemoryCategory, Vec<MemoryId>>,
    pub(crate) by_tag: std::collections::HashMap<String, Vec<MemoryId>>,
}

impl MemoryIndex {
    pub(crate) fn add(&mut self, memory: &Memory) {
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

    pub(crate) fn remove(&mut self, memory: &Memory) {
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

    pub(crate) fn clear(&mut self) {
        self.by_type.clear();
        self.by_category.clear();
        self.by_tag.clear();
    }
}

/// Memory manager
pub struct MemoryManager {
    pub(crate) storage: Arc<dyn MemoryStorage>,
    pub(crate) config: MemoryConfig,
    pub(crate) index: Arc<RwLock<MemoryIndex>>,
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

    pub(crate) async fn rebuild_index(&self) -> Result<(), MemoryStorageError> {
        let all = self.storage.list(0, self.config.max_memories).await?;
        let mut index = self.index.write().await;
        index.clear();

        for memory in all {
            index.add(&memory);
        }

        Ok(())
    }
}
