//! Workspace analysis and project detection
//!
//! Provides utilities for analyzing project structure, detecting project types,
//! and discovering important files and patterns.

pub mod analyzer;
pub mod dependencies;
pub mod detector;
pub mod entry_points;
pub mod git;
pub mod models;
pub mod patterns;
pub mod statistics;
pub mod structure;

// Re-export commonly used types from models
pub use models::{
    AnalysisResult, DependencyInfo, EntryPoint, FileStats, GitInfo, ProjectStructure,
    WorkspaceConfig, WorkspaceError,
};

// Re-export analyzer
pub use analyzer::WorkspaceAnalyzer;

// Re-export detector types
pub use detector::{
    BuildSystem, FrameworkType, LanguageType, ProjectType, ProjectTypeDetector, RuntimeType,
    TestFramework,
};

// Re-export pattern types
pub use patterns::{ImportantFile, ImportantFileType, PatternMatcher, ProjectPattern};
