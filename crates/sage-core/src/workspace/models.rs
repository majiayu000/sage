//! Data models for workspace analysis

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use super::detector::ProjectType;
use super::patterns::ImportantFile;

/// Workspace analysis error
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

/// Workspace analyzer configuration
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    /// Maximum directory depth for scanning
    pub max_depth: usize,
    /// Maximum number of files to scan
    pub max_files: usize,
    /// Include hidden files
    pub include_hidden: bool,
    /// Exclude patterns (glob)
    pub exclude_patterns: Vec<String>,
    /// Enable dependency analysis
    pub analyze_dependencies: bool,
    /// Enable git analysis
    pub analyze_git: bool,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_files: 10000,
            include_hidden: false,
            exclude_patterns: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "__pycache__".to_string(),
                "venv".to_string(),
                ".venv".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".next".to_string(),
                ".nuxt".to_string(),
                "coverage".to_string(),
            ],
            analyze_dependencies: true,
            analyze_git: true,
        }
    }
}

/// File statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileStats {
    /// Total number of files
    pub total_files: usize,
    /// Total number of directories
    pub total_directories: usize,
    /// Files by extension
    pub by_extension: HashMap<String, usize>,
    /// Files by language
    pub by_language: HashMap<String, usize>,
    /// Total lines of code (estimated)
    pub total_lines: usize,
    /// Largest files (path, size)
    pub largest_files: Vec<(PathBuf, u64)>,
}

/// Entry point information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    /// File path
    pub path: PathBuf,
    /// Entry point type (main, lib, server, etc.)
    pub entry_type: String,
    /// Primary function/export
    pub primary: Option<String>,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Direct dependencies
    pub dependencies: Vec<String>,
    /// Dev dependencies
    pub dev_dependencies: Vec<String>,
    /// Peer dependencies (for Node.js)
    pub peer_dependencies: Vec<String>,
    /// Dependency count
    pub total_count: usize,
    /// Has lock file
    pub has_lock_file: bool,
    /// Lock file path
    pub lock_file: Option<PathBuf>,
}

/// Project structure analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    /// Source directories
    pub source_dirs: Vec<PathBuf>,
    /// Test directories
    pub test_dirs: Vec<PathBuf>,
    /// Documentation directories
    pub doc_dirs: Vec<PathBuf>,
    /// Build output directories
    pub build_dirs: Vec<PathBuf>,
    /// Config directory
    pub config_dir: Option<PathBuf>,
    /// Has conventional structure
    pub is_conventional: bool,
}

/// Git repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    /// Is a git repository
    pub is_repo: bool,
    /// Current branch
    pub branch: Option<String>,
    /// Remote URL
    pub remote_url: Option<String>,
    /// Has uncommitted changes
    pub has_changes: bool,
    /// Total commits (approximate)
    pub commit_count: Option<usize>,
}

/// Complete analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Root directory
    pub root: PathBuf,
    /// Project type detection
    pub project_type: ProjectType,
    /// Important files
    pub important_files: Vec<ImportantFile>,
    /// File statistics
    pub stats: FileStats,
    /// Entry points
    pub entry_points: Vec<EntryPoint>,
    /// Dependencies
    pub dependencies: Option<DependencyInfo>,
    /// Project structure
    pub structure: ProjectStructure,
    /// Git information
    pub git_info: Option<GitInfo>,
    /// Analysis duration
    pub analysis_duration_ms: u64,
}

impl AnalysisResult {
    /// Get a summary string
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "Project: {} ({:.0}% confidence)",
            self.project_type.primary_language.name(),
            self.project_type.confidence * 100.0
        ));

        if !self.project_type.frameworks.is_empty() {
            let frameworks: Vec<_> = self
                .project_type
                .frameworks
                .iter()
                .map(|f| f.name())
                .collect();
            lines.push(format!("Frameworks: {}", frameworks.join(", ")));
        }

        if !self.project_type.build_systems.is_empty() {
            let build: Vec<_> = self
                .project_type
                .build_systems
                .iter()
                .map(|b| b.name())
                .collect();
            lines.push(format!("Build: {}", build.join(", ")));
        }

        lines.push(format!(
            "Files: {} ({} directories)",
            self.stats.total_files, self.stats.total_directories
        ));

        if let Some(deps) = &self.dependencies {
            lines.push(format!(
                "Dependencies: {} direct, {} dev",
                deps.dependencies.len(),
                deps.dev_dependencies.len()
            ));
        }

        if !self.entry_points.is_empty() {
            lines.push(format!("Entry points: {}", self.entry_points.len()));
        }

        if let Some(git) = &self.git_info {
            if let Some(branch) = &git.branch {
                lines.push(format!("Git branch: {}", branch));
            }
        }

        lines.push(format!("Analysis took: {}ms", self.analysis_duration_ms));

        lines.join("\n")
    }
}
