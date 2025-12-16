//! Workspace analyzer for comprehensive project analysis
//!
//! Provides detailed analysis of project structure, dependencies, and statistics.

use super::detector::{LanguageType, ProjectType, ProjectTypeDetector};
use super::patterns::{ImportantFile, ImportantFileType, PatternMatcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;

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

/// Workspace analyzer
pub struct WorkspaceAnalyzer {
    root: PathBuf,
    config: WorkspaceConfig,
}

impl WorkspaceAnalyzer {
    /// Create a new analyzer
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            config: WorkspaceConfig::default(),
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: WorkspaceConfig) -> Self {
        self.config = config;
        self
    }

    /// Perform full analysis
    pub fn analyze(&self) -> Result<AnalysisResult, WorkspaceError> {
        let start = Instant::now();

        // Validate root
        if !self.root.exists() {
            return Err(WorkspaceError::DirectoryNotFound(self.root.clone()));
        }
        if !self.root.is_dir() {
            return Err(WorkspaceError::NotADirectory(self.root.clone()));
        }

        // Detect project type
        let project_type = ProjectTypeDetector::new(&self.root)
            .with_max_depth(self.config.max_depth)
            .detect();

        // Find important files
        let matcher = PatternMatcher::for_language(project_type.primary_language);
        let important_files = matcher.find_important_files(&self.root);

        // Collect file statistics
        let stats = self.collect_stats()?;

        // Find entry points
        let entry_points = self.find_entry_points(&important_files, &project_type);

        // Analyze dependencies
        let dependencies = if self.config.analyze_dependencies {
            self.analyze_dependencies(&project_type)
        } else {
            None
        };

        // Analyze structure
        let structure = self.analyze_structure(&project_type);

        // Git info
        let git_info = if self.config.analyze_git {
            self.get_git_info()
        } else {
            None
        };

        Ok(AnalysisResult {
            root: self.root.clone(),
            project_type,
            important_files,
            stats,
            entry_points,
            dependencies,
            structure,
            git_info,
            analysis_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Quick detection (just project type)
    pub fn detect_type(&self) -> ProjectType {
        ProjectTypeDetector::new(&self.root)
            .with_max_depth(self.config.max_depth)
            .detect()
    }

    fn collect_stats(&self) -> Result<FileStats, WorkspaceError> {
        let mut stats = FileStats::default();
        let mut files_scanned = 0;

        self.scan_directory(&self.root, 0, &mut stats, &mut files_scanned)?;

        // Sort largest files
        stats.largest_files.sort_by(|a, b| b.1.cmp(&a.1));
        stats.largest_files.truncate(10);

        Ok(stats)
    }

    fn scan_directory(
        &self,
        dir: &Path,
        depth: usize,
        stats: &mut FileStats,
        files_scanned: &mut usize,
    ) -> Result<(), WorkspaceError> {
        if depth > self.config.max_depth || *files_scanned >= self.config.max_files {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip hidden files if configured
            if !self.config.include_hidden && file_name.starts_with('.') {
                continue;
            }

            // Skip excluded patterns
            if self.should_exclude(file_name) {
                continue;
            }

            if path.is_dir() {
                stats.total_directories += 1;
                self.scan_directory(&path, depth + 1, stats, files_scanned)?;
            } else if path.is_file() {
                stats.total_files += 1;
                *files_scanned += 1;

                // Count by extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    *stats.by_extension.entry(ext.to_lowercase()).or_default() += 1;

                    // Map extension to language
                    if let Some(lang) = self.extension_to_language(ext) {
                        *stats.by_language.entry(lang.to_string()).or_default() += 1;
                    }
                }

                // Track file size
                if let Ok(metadata) = path.metadata() {
                    let size = metadata.len();
                    if stats.largest_files.len() < 10 || size > stats.largest_files.last().map(|f| f.1).unwrap_or(0) {
                        let relative = path.strip_prefix(&self.root).unwrap_or(&path).to_path_buf();
                        stats.largest_files.push((relative, size));
                    }

                    // Estimate lines (rough: 40 bytes per line average)
                    stats.total_lines += (size / 40) as usize;
                }
            }
        }

        Ok(())
    }

    fn should_exclude(&self, name: &str) -> bool {
        self.config.exclude_patterns.iter().any(|p| {
            if p.contains('*') {
                glob::Pattern::new(p).map(|pat| pat.matches(name)).unwrap_or(false)
            } else {
                name == p
            }
        })
    }

    fn extension_to_language(&self, ext: &str) -> Option<&str> {
        match ext.to_lowercase().as_str() {
            "rs" => Some("Rust"),
            "ts" | "tsx" => Some("TypeScript"),
            "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript"),
            "py" | "pyi" => Some("Python"),
            "go" => Some("Go"),
            "java" => Some("Java"),
            "kt" | "kts" => Some("Kotlin"),
            "scala" | "sc" => Some("Scala"),
            "cs" => Some("C#"),
            "cpp" | "cc" | "cxx" | "hpp" => Some("C++"),
            "c" | "h" => Some("C"),
            "rb" => Some("Ruby"),
            "php" => Some("PHP"),
            "swift" => Some("Swift"),
            "sh" | "bash" | "zsh" => Some("Shell"),
            "sql" => Some("SQL"),
            "html" | "htm" => Some("HTML"),
            "css" | "scss" | "sass" | "less" => Some("CSS"),
            "json" => Some("JSON"),
            "yaml" | "yml" => Some("YAML"),
            "toml" => Some("TOML"),
            "md" | "markdown" => Some("Markdown"),
            _ => None,
        }
    }

    fn find_entry_points(&self, important_files: &[ImportantFile], project: &ProjectType) -> Vec<EntryPoint> {
        important_files
            .iter()
            .filter(|f| f.file_type == ImportantFileType::EntryPoint)
            .map(|f| {
                let entry_type = self.determine_entry_type(&f.path, project);
                EntryPoint {
                    path: f.path.clone(),
                    entry_type,
                    primary: None,
                }
            })
            .collect()
    }

    fn determine_entry_type(&self, path: &Path, project: &ProjectType) -> String {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match project.primary_language {
            LanguageType::Rust => {
                if file_name == "main.rs" {
                    "main".to_string()
                } else if file_name == "lib.rs" {
                    "library".to_string()
                } else {
                    "binary".to_string()
                }
            }
            LanguageType::TypeScript | LanguageType::JavaScript => {
                if file_name.contains("server") {
                    "server".to_string()
                } else if file_name.contains("app") || file_name.contains("App") {
                    "app".to_string()
                } else if file_name.contains("index") {
                    "index".to_string()
                } else {
                    "module".to_string()
                }
            }
            LanguageType::Python => {
                if file_name == "__main__.py" {
                    "main".to_string()
                } else if file_name == "__init__.py" {
                    "package".to_string()
                } else if file_name == "app.py" {
                    "app".to_string()
                } else {
                    "module".to_string()
                }
            }
            LanguageType::Go => {
                if path.to_string_lossy().contains("cmd/") {
                    "command".to_string()
                } else {
                    "main".to_string()
                }
            }
            _ => "entry".to_string(),
        }
    }

    fn analyze_dependencies(&self, project: &ProjectType) -> Option<DependencyInfo> {
        let mut info = DependencyInfo {
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            peer_dependencies: Vec::new(),
            total_count: 0,
            has_lock_file: false,
            lock_file: None,
        };

        match project.primary_language {
            LanguageType::Rust => {
                self.parse_cargo_dependencies(&mut info);
            }
            LanguageType::TypeScript | LanguageType::JavaScript => {
                self.parse_npm_dependencies(&mut info);
            }
            LanguageType::Python => {
                self.parse_python_dependencies(&mut info);
            }
            LanguageType::Go => {
                self.parse_go_dependencies(&mut info);
            }
            _ => return None,
        }

        info.total_count = info.dependencies.len() + info.dev_dependencies.len();
        Some(info)
    }

    fn parse_cargo_dependencies(&self, info: &mut DependencyInfo) {
        let cargo_toml = self.root.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            // Simple parsing - look for dependencies sections
            let mut in_deps = false;
            let mut in_dev_deps = false;

            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("[dependencies]") {
                    in_deps = true;
                    in_dev_deps = false;
                } else if line.starts_with("[dev-dependencies]") {
                    in_deps = false;
                    in_dev_deps = true;
                } else if line.starts_with('[') {
                    in_deps = false;
                    in_dev_deps = false;
                } else if !line.is_empty() && !line.starts_with('#') {
                    if let Some(name) = line.split('=').next().map(|s| s.trim().to_string()) {
                        if in_deps {
                            info.dependencies.push(name);
                        } else if in_dev_deps {
                            info.dev_dependencies.push(name);
                        }
                    }
                }
            }
        }

        // Check for lock file
        let lock = self.root.join("Cargo.lock");
        if lock.exists() {
            info.has_lock_file = true;
            info.lock_file = Some(PathBuf::from("Cargo.lock"));
        }
    }

    fn parse_npm_dependencies(&self, info: &mut DependencyInfo) {
        let package_json = self.root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&package_json) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                    info.dependencies = deps.keys().cloned().collect();
                }
                if let Some(deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
                    info.dev_dependencies = deps.keys().cloned().collect();
                }
                if let Some(deps) = json.get("peerDependencies").and_then(|d| d.as_object()) {
                    info.peer_dependencies = deps.keys().cloned().collect();
                }
            }
        }

        // Check for lock files
        let lock_files = [
            "package-lock.json",
            "yarn.lock",
            "pnpm-lock.yaml",
            "bun.lockb",
        ];
        for lock in lock_files {
            if self.root.join(lock).exists() {
                info.has_lock_file = true;
                info.lock_file = Some(PathBuf::from(lock));
                break;
            }
        }
    }

    fn parse_python_dependencies(&self, info: &mut DependencyInfo) {
        // Try pyproject.toml first
        let pyproject = self.root.join("pyproject.toml");
        if pyproject.exists() {
            if let Ok(content) = std::fs::read_to_string(&pyproject) {
                // Simple parsing for dependencies
                let mut in_deps = false;
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("dependencies") || line.contains("[tool.poetry.dependencies]") {
                        in_deps = true;
                    } else if line.starts_with('[') && !line.contains("dependencies") {
                        in_deps = false;
                    } else if in_deps && !line.is_empty() && !line.starts_with('#') {
                        if let Some(name) = line.split('=').next().map(|s| s.trim().trim_matches('"').to_string()) {
                            if !name.starts_with('[') && name != "python" {
                                info.dependencies.push(name);
                            }
                        }
                    }
                }
            }
        }

        // Try requirements.txt
        let requirements = self.root.join("requirements.txt");
        if requirements.exists() {
            if let Ok(content) = std::fs::read_to_string(&requirements) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') && !line.starts_with('-') {
                        let name = line.split(|c| c == '=' || c == '<' || c == '>' || c == '[')
                            .next()
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        if !name.is_empty() {
                            info.dependencies.push(name);
                        }
                    }
                }
            }
        }

        // Check for lock files
        let lock_files = ["poetry.lock", "Pipfile.lock", "pdm.lock", "uv.lock"];
        for lock in lock_files {
            if self.root.join(lock).exists() {
                info.has_lock_file = true;
                info.lock_file = Some(PathBuf::from(lock));
                break;
            }
        }
    }

    fn parse_go_dependencies(&self, info: &mut DependencyInfo) {
        let go_mod = self.root.join("go.mod");
        if let Ok(content) = std::fs::read_to_string(&go_mod) {
            let mut in_require = false;
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("require (") {
                    in_require = true;
                } else if line == ")" {
                    in_require = false;
                } else if in_require || line.starts_with("require ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let name = parts[0].trim_start_matches("require ");
                        info.dependencies.push(name.to_string());
                    }
                }
            }
        }

        // Check for lock file
        if self.root.join("go.sum").exists() {
            info.has_lock_file = true;
            info.lock_file = Some(PathBuf::from("go.sum"));
        }
    }

    fn analyze_structure(&self, project: &ProjectType) -> ProjectStructure {
        let mut structure = ProjectStructure {
            source_dirs: Vec::new(),
            test_dirs: Vec::new(),
            doc_dirs: Vec::new(),
            build_dirs: Vec::new(),
            config_dir: None,
            is_conventional: false,
        };

        // Common source directories
        let source_candidates: Vec<&str> = match project.primary_language {
            LanguageType::Rust => vec!["src", "crates"],
            LanguageType::TypeScript | LanguageType::JavaScript => vec!["src", "lib", "app", "pages", "components"],
            LanguageType::Python => vec!["src", "lib"],
            LanguageType::Go => vec!["cmd", "pkg", "internal"],
            LanguageType::Java => vec!["src/main/java", "src/main/kotlin"],
            _ => vec!["src", "lib"],
        };

        for candidate in source_candidates {
            let path = self.root.join(candidate);
            if path.exists() && path.is_dir() {
                structure.source_dirs.push(PathBuf::from(candidate));
            }
        }

        // Test directories
        let test_candidates = match project.primary_language {
            LanguageType::Rust => vec!["tests"],
            LanguageType::TypeScript | LanguageType::JavaScript => vec!["tests", "test", "__tests__", "spec"],
            LanguageType::Python => vec!["tests", "test"],
            LanguageType::Go => vec![], // Go tests are in same dir
            LanguageType::Java => vec!["src/test/java", "src/test/kotlin"],
            _ => vec!["tests", "test"],
        };

        for candidate in test_candidates {
            let path = self.root.join(candidate);
            if path.exists() && path.is_dir() {
                structure.test_dirs.push(PathBuf::from(candidate));
            }
        }

        // Documentation directories
        for candidate in ["docs", "doc", "documentation"] {
            let path = self.root.join(candidate);
            if path.exists() && path.is_dir() {
                structure.doc_dirs.push(PathBuf::from(candidate));
            }
        }

        // Build directories (usually gitignored)
        let build_candidates = match project.primary_language {
            LanguageType::Rust => vec!["target"],
            LanguageType::TypeScript | LanguageType::JavaScript => vec!["dist", "build", ".next", ".nuxt", "out"],
            LanguageType::Python => vec!["dist", "build", "__pycache__"],
            LanguageType::Go => vec!["bin"],
            LanguageType::Java => vec!["target", "build", "out"],
            _ => vec!["dist", "build"],
        };

        for candidate in build_candidates {
            let path = self.root.join(candidate);
            if path.exists() && path.is_dir() {
                structure.build_dirs.push(PathBuf::from(candidate));
            }
        }

        // Config directory
        for candidate in [".config", "config", ".sage", ".claude"] {
            let path = self.root.join(candidate);
            if path.exists() && path.is_dir() {
                structure.config_dir = Some(PathBuf::from(candidate));
                break;
            }
        }

        // Check if conventional structure
        structure.is_conventional = !structure.source_dirs.is_empty();

        structure
    }

    fn get_git_info(&self) -> Option<GitInfo> {
        let git_dir = self.root.join(".git");
        if !git_dir.exists() {
            return None;
        }

        let mut info = GitInfo {
            is_repo: true,
            branch: None,
            remote_url: None,
            has_changes: false,
            commit_count: None,
        };

        // Get current branch
        let head = git_dir.join("HEAD");
        if let Ok(content) = std::fs::read_to_string(head) {
            if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
                info.branch = Some(branch.trim().to_string());
            }
        }

        // Get remote URL
        let config = git_dir.join("config");
        if let Ok(content) = std::fs::read_to_string(config) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("url = ") {
                    info.remote_url = Some(line.strip_prefix("url = ").unwrap().to_string());
                    break;
                }
            }
        }

        Some(info)
    }
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
            let frameworks: Vec<_> = self.project_type.frameworks.iter().map(|f| f.name()).collect();
            lines.push(format!("Frameworks: {}", frameworks.join(", ")));
        }

        if !self.project_type.build_systems.is_empty() {
            let build: Vec<_> = self.project_type.build_systems.iter().map(|b| b.name()).collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_analyzer_creation() {
        let temp = TempDir::new().unwrap();
        let analyzer = WorkspaceAnalyzer::new(temp.path());
        assert_eq!(analyzer.root, temp.path());
    }

    #[test]
    fn test_analyze_rust_project() {
        let temp = TempDir::new().unwrap();

        // Create Rust project structure
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
tokio = "1.0"
serde = "1.0"

[dev-dependencies]
tempfile = "3.0"
"#,
        )
        .unwrap();

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub fn lib() {}").unwrap();

        fs::create_dir_all(temp.path().join("tests")).unwrap();
        fs::write(temp.path().join("tests/test.rs"), "#[test] fn test() {}").unwrap();

        fs::write(temp.path().join("README.md"), "# Test").unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path());
        let result = analyzer.analyze().unwrap();

        assert_eq!(result.project_type.primary_language, LanguageType::Rust);
        assert!(result.project_type.confidence > 0.5);

        // Check dependencies
        let deps = result.dependencies.unwrap();
        assert!(deps.dependencies.contains(&"tokio".to_string()));
        assert!(deps.dependencies.contains(&"serde".to_string()));
        assert!(deps.dev_dependencies.contains(&"tempfile".to_string()));

        // Check entry points
        assert!(!result.entry_points.is_empty());

        // Check structure
        assert!(result.structure.source_dirs.contains(&PathBuf::from("src")));
        assert!(result.structure.test_dirs.contains(&PathBuf::from("tests")));
    }

    #[test]
    fn test_analyze_node_project() {
        let temp = TempDir::new().unwrap();

        fs::write(
            temp.path().join("package.json"),
            r#"
{
    "name": "test",
    "dependencies": {
        "react": "^18.0.0",
        "axios": "^1.0.0"
    },
    "devDependencies": {
        "typescript": "^5.0.0"
    }
}
"#,
        )
        .unwrap();

        fs::write(temp.path().join("tsconfig.json"), "{}").unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/index.ts"), "").unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path());
        let result = analyzer.analyze().unwrap();

        assert_eq!(result.project_type.primary_language, LanguageType::TypeScript);

        let deps = result.dependencies.unwrap();
        assert!(deps.dependencies.contains(&"react".to_string()));
        assert!(deps.dev_dependencies.contains(&"typescript".to_string()));
    }

    #[test]
    fn test_file_stats() {
        let temp = TempDir::new().unwrap();

        fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp.path().join("lib.rs"), "pub fn lib() {}").unwrap();
        fs::write(temp.path().join("test.ts"), "").unwrap();
        fs::create_dir(temp.path().join("subdir")).unwrap();
        fs::write(temp.path().join("subdir/file.rs"), "").unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path());
        let result = analyzer.analyze().unwrap();

        assert!(result.stats.total_files >= 4);
        assert!(result.stats.total_directories >= 1);
        assert!(result.stats.by_extension.get("rs").unwrap_or(&0) >= &3);
    }

    #[test]
    fn test_config_options() {
        let config = WorkspaceConfig {
            max_depth: 5,
            max_files: 1000,
            include_hidden: true,
            exclude_patterns: vec!["*.log".to_string()],
            analyze_dependencies: false,
            analyze_git: false,
        };

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path()).with_config(config);
        let result = analyzer.analyze().unwrap();

        assert!(result.dependencies.is_none()); // Dependencies analysis disabled
        assert!(result.git_info.is_none()); // Git analysis disabled
    }

    #[test]
    fn test_detect_type_only() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("go.mod"), "module test\n\ngo 1.21").unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path());
        let project_type = analyzer.detect_type();

        assert_eq!(project_type.primary_language, LanguageType::Go);
    }

    #[test]
    fn test_git_info() {
        let temp = TempDir::new().unwrap();

        // Create fake .git directory
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::write(temp.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(
            temp.path().join(".git/config"),
            "[remote \"origin\"]\n    url = https://github.com/test/test.git\n",
        )
        .unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path());
        let result = analyzer.analyze().unwrap();

        let git_info = result.git_info.unwrap();
        assert!(git_info.is_repo);
        assert_eq!(git_info.branch, Some("main".to_string()));
        assert!(git_info.remote_url.is_some());
    }

    #[test]
    fn test_analysis_summary() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/main.rs"), "fn main() {}").unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path());
        let result = analyzer.analyze().unwrap();

        let summary = result.summary();
        assert!(summary.contains("Rust"));
        assert!(summary.contains("Files:"));
    }

    #[test]
    fn test_error_handling() {
        let analyzer = WorkspaceAnalyzer::new("/nonexistent/path");
        let result = analyzer.analyze();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorkspaceError::DirectoryNotFound(_)));
    }

    #[test]
    fn test_exclude_patterns() {
        let temp = TempDir::new().unwrap();

        // Create excluded directory
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        fs::write(temp.path().join("node_modules/dep.js"), "").unwrap();

        // Create regular directory
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/main.js"), "").unwrap();

        let analyzer = WorkspaceAnalyzer::new(temp.path());
        let result = analyzer.analyze().unwrap();

        // node_modules should be excluded
        assert_eq!(result.stats.total_files, 1); // Only main.js
    }
}
