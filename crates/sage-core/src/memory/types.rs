//! Memory types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique memory identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub String);

impl MemoryId {
    /// Create a new random memory ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the ID string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Memory type/category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    /// Factual information (e.g., "The project uses Rust 1.70")
    Fact,
    /// User preference (e.g., "User prefers tabs over spaces")
    Preference,
    /// Code context (e.g., "Main entry point is src/main.rs")
    CodeContext,
    /// Conversation summary
    ConversationSummary,
    /// Task/action taken
    TaskHistory,
    /// Error/lesson learned
    Lesson,
    /// Custom type
    Custom,
}

impl MemoryType {
    /// Get display name
    pub fn name(&self) -> &str {
        match self {
            Self::Fact => "Fact",
            Self::Preference => "Preference",
            Self::CodeContext => "Code Context",
            Self::ConversationSummary => "Conversation",
            Self::TaskHistory => "Task",
            Self::Lesson => "Lesson",
            Self::Custom => "Custom",
        }
    }
}

/// Memory category for organization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryCategory {
    /// Project-level memory
    Project,
    /// Session-level memory
    Session,
    /// Global/user-level memory
    Global,
    /// Tool-specific memory
    Tool(String),
    /// Custom category
    Custom(String),
}

impl MemoryCategory {
    /// Get display name
    pub fn name(&self) -> String {
        match self {
            Self::Project => "Project".to_string(),
            Self::Session => "Session".to_string(),
            Self::Global => "Global".to_string(),
            Self::Tool(name) => format!("Tool:{}", name),
            Self::Custom(name) => name.clone(),
        }
    }
}

/// Memory source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemorySource {
    /// From user input
    User,
    /// From agent inference
    Agent,
    /// From tool output
    Tool(String),
    /// From file analysis
    File(String),
    /// Imported from external source
    Import,
    /// System-generated
    System,
}

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
    fn test_memory_id() {
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        assert_ne!(id1, id2);

        let id3 = MemoryId::from_string("test-id");
        assert_eq!(id3.as_str(), "test-id");
    }

    #[test]
    fn test_memory_type_name() {
        assert_eq!(MemoryType::Fact.name(), "Fact");
        assert_eq!(MemoryType::Preference.name(), "Preference");
    }

    #[test]
    fn test_memory_category_name() {
        assert_eq!(MemoryCategory::Project.name(), "Project");
        assert_eq!(MemoryCategory::Tool("bash".to_string()).name(), "Tool:bash");
    }

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

    #[test]
    fn test_memory_creation() {
        let memory = Memory::fact("Rust uses Cargo as its build system");

        assert_eq!(memory.memory_type, MemoryType::Fact);
        assert_eq!(memory.category, MemoryCategory::Project);
        assert!(memory.content.contains("Cargo"));
    }

    #[test]
    fn test_memory_preference() {
        let memory = Memory::preference("User prefers dark mode")
            .with_category(MemoryCategory::Global);

        assert_eq!(memory.memory_type, MemoryType::Preference);
        assert_eq!(memory.category, MemoryCategory::Global);
    }

    #[test]
    fn test_memory_with_data() {
        let data = serde_json::json!({
            "file": "Cargo.toml",
            "section": "dependencies"
        });

        let memory = Memory::code_context("Project dependencies")
            .with_data(data.clone());

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
        let memory = Memory::fact("Pinned fact")
            .with_metadata(MemoryMetadata::default().with_pinned(true));

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
