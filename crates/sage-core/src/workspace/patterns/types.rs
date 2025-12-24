//! Type definitions for project patterns

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Type of important file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImportantFileType {
    /// Entry point (main, index, etc.)
    EntryPoint,
    /// Configuration file
    Config,
    /// Build file
    Build,
    /// Test file
    Test,
    /// Documentation
    Documentation,
    /// CI/CD configuration
    CiCd,
    /// Docker/container
    Container,
    /// Environment/secrets
    Environment,
    /// Lock file
    LockFile,
    /// Type definitions
    TypeDefinition,
    /// API definition
    ApiDefinition,
    /// Database
    Database,
    /// License
    License,
}

impl ImportantFileType {
    /// Get display name
    pub fn name(&self) -> &str {
        match self {
            Self::EntryPoint => "Entry Point",
            Self::Config => "Configuration",
            Self::Build => "Build",
            Self::Test => "Test",
            Self::Documentation => "Documentation",
            Self::CiCd => "CI/CD",
            Self::Container => "Container",
            Self::Environment => "Environment",
            Self::LockFile => "Lock File",
            Self::TypeDefinition => "Type Definition",
            Self::ApiDefinition => "API Definition",
            Self::Database => "Database",
            Self::License => "License",
        }
    }
}

/// An important file in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportantFile {
    /// File path relative to project root
    pub path: PathBuf,
    /// File type
    pub file_type: ImportantFileType,
    /// File description
    pub description: Option<String>,
    /// Priority (higher = more important)
    pub priority: u32,
}

/// Project pattern definition
#[derive(Debug, Clone)]
pub struct ProjectPattern {
    /// Pattern name
    pub name: String,
    /// File patterns (glob-style)
    pub patterns: Vec<String>,
    /// File type
    pub file_type: ImportantFileType,
    /// Priority
    pub priority: u32,
    /// Optional description
    pub description: Option<String>,
}

impl ProjectPattern {
    /// Create a new pattern
    pub fn new(name: impl Into<String>, file_type: ImportantFileType) -> Self {
        Self {
            name: name.into(),
            patterns: Vec::new(),
            file_type,
            priority: 50,
            description: None,
        }
    }

    /// Add patterns
    pub fn with_patterns(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.patterns = patterns.into_iter().map(|p| p.into()).collect();
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
