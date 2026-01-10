//! Skill metadata types

use serde::{Deserialize, Serialize};

/// Skill metadata (name, display_name, description, version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Skill name (used for invocation, e.g., "commit" for /commit)
    pub name: String,

    /// Display name (human-readable)
    pub display_name: Option<String>,

    /// Short description
    pub description: String,

    /// Skill version
    pub version: Option<String>,
}

impl SkillMetadata {
    /// Create new skill metadata
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: None,
            description: description.into(),
            version: None,
        }
    }

    /// Get the user-facing name (display_name or name)
    pub fn user_facing_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }
}
