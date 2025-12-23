//! Type definitions module

mod build_system;
mod framework;
mod language;
mod runtime;
mod test_framework;

pub use build_system::BuildSystem;
pub use framework::FrameworkType;
pub use language::LanguageType;
pub use runtime::RuntimeType;
pub use test_framework::TestFramework;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Detected project type with all metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectType {
    /// Primary language
    pub primary_language: LanguageType,
    /// Secondary languages
    pub secondary_languages: HashSet<LanguageType>,
    /// Detected frameworks
    pub frameworks: HashSet<FrameworkType>,
    /// Build systems
    pub build_systems: HashSet<BuildSystem>,
    /// Test frameworks
    pub test_frameworks: HashSet<TestFramework>,
    /// Runtime type
    pub runtime: Option<RuntimeType>,
    /// Is it a monorepo?
    pub is_monorepo: bool,
    /// Is it a workspace?
    pub is_workspace: bool,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

impl Default for ProjectType {
    fn default() -> Self {
        Self {
            primary_language: LanguageType::Unknown,
            secondary_languages: HashSet::new(),
            frameworks: HashSet::new(),
            build_systems: HashSet::new(),
            test_frameworks: HashSet::new(),
            runtime: None,
            is_monorepo: false,
            is_workspace: false,
            confidence: 0.0,
        }
    }
}
