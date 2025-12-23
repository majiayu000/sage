//! Workspace analyzer for comprehensive project analysis
//!
//! Provides detailed analysis of project structure, dependencies, and statistics.

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::dependencies;
use super::detector::{LanguageType, ProjectTypeDetector};
use super::entry_points;
use super::git;
use super::models::{AnalysisResult, FileStats, WorkspaceConfig, WorkspaceError};
use super::patterns::PatternMatcher;
use super::statistics;
use super::structure;

/// Workspace analyzer
#[derive(Clone)]
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

        // Detect project type (fast, without extra filesystem scans)
        let detector = ProjectTypeDetector::new(&self.root).with_max_depth(self.config.max_depth);
        let mut project_type = detector.detect_fast();

        // Find important files
        let matcher = PatternMatcher::for_language(project_type.primary_language);
        let important_files = matcher.find_important_files(&self.root);

        // Collect file statistics
        let stats = statistics::collect_stats(&self.root, &self.config)?;

        if project_type.primary_language == LanguageType::Unknown {
            if let Some(lang) = primary_language_from_stats(&stats) {
                project_type.primary_language = lang;
                detector.recalculate_confidence(&mut project_type);
            }
        }

        // Find entry points
        let entry_points = entry_points::find_entry_points(&important_files, &project_type);

        // Analyze dependencies
        let dependencies = if self.config.analyze_dependencies {
            dependencies::analyze_dependencies(&self.root, &project_type)
        } else {
            None
        };

        // Analyze structure
        let structure = structure::analyze_structure(&self.root, &project_type);

        // Git info
        let git_info = if self.config.analyze_git {
            git::get_git_info(&self.root)
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

    /// Perform full analysis on a blocking thread
    pub async fn analyze_async(&self) -> Result<AnalysisResult, WorkspaceError> {
        let analyzer = self.clone();
        tokio::task::spawn_blocking(move || analyzer.analyze())
            .await
            .map_err(|e| WorkspaceError::AnalysisFailed(format!("Analysis task failed: {}", e)))?
    }

    /// Quick detection (just project type)
    pub fn detect_type(&self) -> super::detector::ProjectType {
        ProjectTypeDetector::new(&self.root)
            .with_max_depth(self.config.max_depth)
            .detect()
    }
}

fn primary_language_from_stats(stats: &FileStats) -> Option<LanguageType> {
    stats
        .by_language
        .iter()
        .max_by_key(|(_, count)| *count)
        .and_then(|(lang, _)| match lang.as_str() {
            "Rust" => Some(LanguageType::Rust),
            "TypeScript" => Some(LanguageType::TypeScript),
            "JavaScript" => Some(LanguageType::JavaScript),
            "Python" => Some(LanguageType::Python),
            "Go" => Some(LanguageType::Go),
            "Java" => Some(LanguageType::Java),
            "C#" => Some(LanguageType::CSharp),
            "C++" => Some(LanguageType::Cpp),
            "C" => Some(LanguageType::C),
            "Ruby" => Some(LanguageType::Ruby),
            "Swift" => Some(LanguageType::Swift),
            "Kotlin" => Some(LanguageType::Kotlin),
            "Scala" => Some(LanguageType::Scala),
            "PHP" => Some(LanguageType::Php),
            "Shell" => Some(LanguageType::Shell),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::super::detector::LanguageType;
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

        assert_eq!(
            result.project_type.primary_language,
            LanguageType::TypeScript
        );

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
        assert!(matches!(
            result.unwrap_err(),
            WorkspaceError::DirectoryNotFound(_)
        ));
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
