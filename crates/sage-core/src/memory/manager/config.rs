//! Memory manager configuration and statistics

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
