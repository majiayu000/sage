//! Workspace analysis and project detection
//!
//! Provides utilities for analyzing project structure, detecting project types,
//! and discovering important files and patterns.

pub mod analyzer;
pub mod detector;
pub mod patterns;

pub use analyzer::{
    AnalysisResult, DependencyInfo, EntryPoint, FileStats, ProjectStructure, WorkspaceAnalyzer,
    WorkspaceConfig, WorkspaceError,
};
pub use detector::{
    BuildSystem, FrameworkType, LanguageType, ProjectType, ProjectTypeDetector, RuntimeType,
    TestFramework,
};
pub use patterns::{ImportantFile, ImportantFileType, PatternMatcher, ProjectPattern};
