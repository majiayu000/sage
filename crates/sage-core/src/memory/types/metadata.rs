//! Memory metadata and source tracking

use super::base::{MemoryId, MemorySource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Source of the memory
    pub source: MemorySource,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub accessed_at: DateTime<Utc>,
    /// Access count
    pub access_count: u32,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Whether this memory is pinned (won't decay)
    pub pinned: bool,
    /// Related memory IDs
    pub related: Vec<MemoryId>,
    /// Custom tags
    pub tags: Vec<String>,
    /// Arbitrary key-value metadata
    pub extra: HashMap<String, String>,
}

impl Default for MemoryMetadata {
    fn default() -> Self {
        Self {
            source: MemorySource::System,
            created_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
            confidence: 1.0,
            pinned: false,
            related: Vec::new(),
            tags: Vec::new(),
            extra: HashMap::new(),
        }
    }
}

impl MemoryMetadata {
    /// Create with source
    pub fn with_source(source: MemorySource) -> Self {
        Self {
            source,
            ..Default::default()
        }
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set pinned
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Mark as accessed
    pub fn touch(&mut self) {
        self.accessed_at = Utc::now();
        self.access_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_metadata() {
        let metadata = MemoryMetadata::with_source(MemorySource::User)
            .with_confidence(0.9)
            .with_pinned(true)
            .with_tags(["rust", "config"]);

        assert!(matches!(metadata.source, MemorySource::User));
        assert_eq!(metadata.confidence, 0.9);
        assert!(metadata.pinned);
        assert_eq!(metadata.tags.len(), 2);
    }

    #[test]
    fn test_memory_metadata_touch() {
        let mut metadata = MemoryMetadata::default();
        let initial_time = metadata.accessed_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        metadata.touch();

        assert!(metadata.accessed_at > initial_time);
        assert_eq!(metadata.access_count, 1);
    }
}
