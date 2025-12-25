//! Memory entries

use super::base::{MemoryCategory, MemoryId, MemoryType};
use super::metadata::MemoryMetadata;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique identifier
    pub id: MemoryId,
    /// Memory type
    pub memory_type: MemoryType,
    /// Category
    pub category: MemoryCategory,
    /// The actual content
    pub content: String,
    /// Optional structured data
    pub data: Option<serde_json::Value>,
    /// Metadata
    pub metadata: MemoryMetadata,
}

impl Memory {
    /// Create a new memory
    pub fn new(
        memory_type: MemoryType,
        category: MemoryCategory,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: MemoryId::new(),
            memory_type,
            category,
            content: content.into(),
            data: None,
            metadata: MemoryMetadata::default(),
        }
    }

    /// Create a fact memory
    pub fn fact(content: impl Into<String>) -> Self {
        Self::new(MemoryType::Fact, MemoryCategory::Project, content)
    }

    /// Create a preference memory
    pub fn preference(content: impl Into<String>) -> Self {
        Self::new(MemoryType::Preference, MemoryCategory::Global, content)
    }

    /// Create a code context memory
    pub fn code_context(content: impl Into<String>) -> Self {
        Self::new(MemoryType::CodeContext, MemoryCategory::Project, content)
    }

    /// Create a lesson memory
    pub fn lesson(content: impl Into<String>) -> Self {
        Self::new(MemoryType::Lesson, MemoryCategory::Global, content)
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: MemoryMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set structured data
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set category
    pub fn with_category(mut self, category: MemoryCategory) -> Self {
        self.category = category;
        self
    }

    /// Add tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.metadata.tags.push(tag.into());
    }

    /// Check if memory matches a tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.metadata.tags.iter().any(|t| t == tag)
    }

    /// Calculate relevance score based on time decay and access frequency
    pub fn relevance_score(&self) -> f32 {
        let age_days = (Utc::now() - self.metadata.created_at).num_days() as f32;
        let recency_days = (Utc::now() - self.metadata.accessed_at).num_days() as f32;

        // Base confidence
        let mut score = self.metadata.confidence;

        // Pinned memories don't decay
        if !self.metadata.pinned {
            // Time decay: lose 10% relevance per week
            let age_decay = 1.0 - (age_days / 70.0).min(0.9);
            score *= age_decay;

            // Recency boost: recently accessed memories are more relevant
            let recency_boost = 1.0 + (7.0 - recency_days.min(7.0)) / 7.0 * 0.3;
            score *= recency_boost;
        }

        // Access frequency boost
        let access_boost = 1.0 + (self.metadata.access_count as f32 / 100.0).min(0.5);
        score *= access_boost;

        score.clamp(0.0, 1.0)
    }
}

/// Relevance score for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RelevanceScore {
    /// Content match score
    pub content_score: f32,
    /// Time decay score
    pub decay_score: f32,
    /// Combined score
    pub total: f32,
}

/// Memory with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryScore {
    /// The memory
    pub memory: Memory,
    /// Relevance score
    pub score: RelevanceScore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let memory = Memory::fact("Rust uses Cargo as its build system");

        assert_eq!(memory.memory_type, MemoryType::Fact);
        assert_eq!(memory.category, MemoryCategory::Project);
        assert!(memory.content.contains("Cargo"));
    }

    #[test]
    fn test_memory_preference() {
        let memory =
            Memory::preference("User prefers dark mode").with_category(MemoryCategory::Global);

        assert_eq!(memory.memory_type, MemoryType::Preference);
        assert_eq!(memory.category, MemoryCategory::Global);
    }

    #[test]
    fn test_memory_with_data() {
        let data = serde_json::json!({
            "file": "Cargo.toml",
            "section": "dependencies"
        });

        let memory = Memory::code_context("Project dependencies").with_data(data.clone());

        assert_eq!(memory.data, Some(data));
    }

    #[test]
    fn test_memory_tags() {
        let mut memory = Memory::fact("Test fact");
        memory.add_tag("important");
        memory.add_tag("rust");

        assert!(memory.has_tag("important"));
        assert!(memory.has_tag("rust"));
        assert!(!memory.has_tag("python"));
    }

    #[test]
    fn test_memory_relevance_pinned() {
        let memory =
            Memory::fact("Pinned fact").with_metadata(MemoryMetadata::default().with_pinned(true));

        // Pinned memories should have high relevance
        assert!(memory.relevance_score() >= 0.9);
    }

    #[test]
    fn test_memory_relevance_fresh() {
        let memory = Memory::fact("Fresh fact");

        // Fresh memories should have high relevance
        assert!(memory.relevance_score() >= 0.9);
    }

    #[test]
    fn test_relevance_score_struct() {
        let score = RelevanceScore {
            content_score: 0.8,
            decay_score: 0.9,
            total: 0.72,
        };

        assert_eq!(score.total, 0.72);
    }

    #[test]
    fn test_memory_score() {
        let memory = Memory::fact("Test");
        let score = MemoryScore {
            memory: memory.clone(),
            score: RelevanceScore {
                content_score: 1.0,
                decay_score: 1.0,
                total: 1.0,
            },
        };

        assert_eq!(score.memory.content, "Test");
    }
}
