//! Helper functions for memory management

use super::config::MemoryConfig;
use super::core::MemoryManager;
use crate::memory::storage::MemoryStorageError;
use std::sync::Arc;

/// Calculate text similarity (simple Jaccard-like metric)
pub(in crate::memory::manager) fn calculate_similarity(a: &str, b: &str) -> f32 {
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
