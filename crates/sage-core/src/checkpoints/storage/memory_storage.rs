//! In-memory checkpoint storage implementation

use crate::error::SageResult;
use async_trait::async_trait;

use super::super::types::{Checkpoint, CheckpointId};
use super::{CheckpointStorage, CheckpointSummary};

/// In-memory checkpoint storage (for testing)
pub struct MemoryCheckpointStorage {
    checkpoints: tokio::sync::RwLock<std::collections::HashMap<String, Checkpoint>>,
    content: tokio::sync::RwLock<std::collections::HashMap<String, String>>,
}

impl MemoryCheckpointStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            checkpoints: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            content: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryCheckpointStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CheckpointStorage for MemoryCheckpointStorage {
    async fn save(&self, checkpoint: &Checkpoint) -> SageResult<()> {
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.insert(checkpoint.id.as_str().to_string(), checkpoint.clone());
        Ok(())
    }

    async fn load(&self, id: &CheckpointId) -> SageResult<Option<Checkpoint>> {
        let checkpoints = self.checkpoints.read().await;
        Ok(checkpoints.get(id.as_str()).cloned())
    }

    async fn list(&self) -> SageResult<Vec<CheckpointSummary>> {
        let checkpoints = self.checkpoints.read().await;
        let mut summaries: Vec<_> = checkpoints.values().map(CheckpointSummary::from).collect();
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(summaries)
    }

    async fn delete(&self, id: &CheckpointId) -> SageResult<()> {
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.remove(id.as_str());
        Ok(())
    }

    async fn exists(&self, id: &CheckpointId) -> SageResult<bool> {
        let checkpoints = self.checkpoints.read().await;
        Ok(checkpoints.contains_key(id.as_str()))
    }

    async fn latest(&self) -> SageResult<Option<Checkpoint>> {
        let summaries = self.list().await?;
        if let Some(summary) = summaries.first() {
            self.load(&summary.id).await
        } else {
            Ok(None)
        }
    }

    async fn store_content(&self, content: &str) -> SageResult<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let content_ref = format!("{:016x}", hasher.finish());

        let mut stored = self.content.write().await;
        stored.insert(content_ref.clone(), content.to_string());
        Ok(content_ref)
    }

    async fn load_content(&self, content_ref: &str) -> SageResult<Option<String>> {
        let stored = self.content.read().await;
        Ok(stored.get(content_ref).cloned())
    }
}
