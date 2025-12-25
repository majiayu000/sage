//! Base memory types and identifiers

use serde::{Deserialize, Serialize};

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
}
