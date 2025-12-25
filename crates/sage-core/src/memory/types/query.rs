//! Memory query and search types

use super::base::{MemoryCategory, MemoryType};
use chrono::{DateTime, Utc};

/// Query for searching memories
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    /// Text to search for
    pub text: Option<String>,
    /// Filter by memory type
    pub memory_type: Option<MemoryType>,
    /// Filter by category
    pub category: Option<MemoryCategory>,
    /// Filter by tags
    pub tags: Vec<String>,
    /// Minimum relevance score
    pub min_relevance: Option<f32>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Include pinned memories
    pub include_pinned: bool,
    /// Only return memories created after this time
    pub created_after: Option<DateTime<Utc>>,
    /// Only return memories accessed after this time
    pub accessed_after: Option<DateTime<Utc>>,
}

impl MemoryQuery {
    /// Create a new query
    pub fn new() -> Self {
        Self {
            include_pinned: true,
            ..Default::default()
        }
    }

    /// Search for text
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Filter by type
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    /// Filter by category
    pub fn category(mut self, category: MemoryCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Filter by tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set minimum relevance
    pub fn min_relevance(mut self, score: f32) -> Self {
        self.min_relevance = Some(score);
        self
    }

    /// Limit results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Only pinned memories
    pub fn pinned_only(mut self) -> Self {
        self.include_pinned = true;
        self
    }

    /// Created after a certain time
    pub fn created_after(mut self, time: DateTime<Utc>) -> Self {
        self.created_after = Some(time);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_query_builder() {
        let query = MemoryQuery::new()
            .text("Rust")
            .memory_type(MemoryType::Fact)
            .category(MemoryCategory::Project)
            .tag("config")
            .min_relevance(0.5)
            .limit(10);

        assert_eq!(query.text, Some("Rust".to_string()));
        assert_eq!(query.memory_type, Some(MemoryType::Fact));
        assert_eq!(query.category, Some(MemoryCategory::Project));
        assert_eq!(query.tags, vec!["config"]);
        assert_eq!(query.min_relevance, Some(0.5));
        assert_eq!(query.limit, Some(10));
    }
}
