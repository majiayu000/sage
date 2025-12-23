//! Project type detection
//!
//! Detects programming languages, frameworks, build systems, and test frameworks
//! based on project files and structure.

mod detectors;
mod framework_detection;
mod language_detection;
mod types;

// Re-export all public types
pub use types::{
    BuildSystem, FrameworkType, LanguageType, ProjectType, RuntimeType, TestFramework,
};

use std::path::Path;

/// Project type detector
pub struct ProjectTypeDetector {
    root: std::path::PathBuf,
    max_depth: usize,
}

impl ProjectTypeDetector {
    /// Create a new detector
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            max_depth: 3,
        }
    }

    /// Set maximum directory depth to scan
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Detect project type
    pub fn detect(&self) -> ProjectType {
        let mut project_type = ProjectType::default();

        // Run all language-specific detectors
        detectors::detect_all(&self.root, &mut project_type);

        // Determine primary language if not set
        if project_type.primary_language == LanguageType::Unknown {
            project_type.primary_language =
                language_detection::detect_by_file_count(&self.root, self.max_depth);
        }

        // Calculate confidence
        project_type.confidence = self.calculate_confidence(&project_type);

        project_type
    }

    fn calculate_confidence(&self, project: &ProjectType) -> f32 {
        let mut confidence: f32 = 0.0;

        // Has a primary language
        if project.primary_language != LanguageType::Unknown {
            confidence += 0.3;
        }

        // Has a build system
        if !project.build_systems.is_empty() {
            confidence += 0.3;
        }

        // Has test frameworks
        if !project.test_frameworks.is_empty() {
            confidence += 0.2;
        }

        // Has frameworks
        if !project.frameworks.is_empty() {
            confidence += 0.2;
        }

        confidence.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_language_extensions() {
        assert_eq!(LanguageType::Rust.extensions(), &["rs"]);
        assert_eq!(LanguageType::TypeScript.extensions(), &["ts", "tsx"]);
        assert_eq!(LanguageType::Python.extensions(), &["py", "pyi"]);
    }

    #[test]
    fn test_language_names() {
        assert_eq!(LanguageType::Rust.name(), "Rust");
        assert_eq!(LanguageType::TypeScript.name(), "TypeScript");
        assert_eq!(LanguageType::CSharp.name(), "C#");
    }

    #[test]
    fn test_build_system_config_file() {
        assert_eq!(BuildSystem::Cargo.config_file(), Some("Cargo.toml"));
        assert_eq!(BuildSystem::Npm.config_file(), Some("package.json"));
        assert_eq!(BuildSystem::GoModules.config_file(), Some("go.mod"));
    }

    #[test]
    fn test_detect_rust_project() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
axum = "0.7"
"#,
        )
        .unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert_eq!(project.primary_language, LanguageType::Rust);
        assert!(project.build_systems.contains(&BuildSystem::Cargo));
        assert!(project.frameworks.contains(&FrameworkType::Axum));
        assert!(
            project
                .test_frameworks
                .contains(&TestFramework::RustBuiltin)
        );
    }

    #[test]
    fn test_detect_rust_workspace() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/*"]
"#,
        )
        .unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert!(project.is_workspace);
    }

    #[test]
    fn test_detect_node_project() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"
{
    "name": "test",
    "dependencies": {
        "react": "^18.0.0",
        "next": "^14.0.0"
    },
    "devDependencies": {
        "jest": "^29.0.0"
    }
}
"#,
        )
        .unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert_eq!(project.primary_language, LanguageType::JavaScript);
        assert!(project.build_systems.contains(&BuildSystem::Npm));
        assert!(project.frameworks.contains(&FrameworkType::React));
        assert!(project.frameworks.contains(&FrameworkType::NextJs));
        assert!(project.test_frameworks.contains(&TestFramework::Jest));
    }

    #[test]
    fn test_detect_typescript_project() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        fs::write(temp.path().join("tsconfig.json"), "{}").unwrap();
        fs::write(temp.path().join("yarn.lock"), "").unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert_eq!(project.primary_language, LanguageType::TypeScript);
        assert!(project.build_systems.contains(&BuildSystem::Yarn));
    }

    #[test]
    fn test_detect_python_project() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("pyproject.toml"),
            r#"
[tool.poetry]
name = "test"

[tool.poetry.dependencies]
django = "^4.0"
pytest = "^7.0"
"#,
        )
        .unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert_eq!(project.primary_language, LanguageType::Python);
        assert!(project.build_systems.contains(&BuildSystem::Poetry));
        assert!(project.frameworks.contains(&FrameworkType::Django));
    }

    #[test]
    fn test_detect_go_project() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("go.mod"),
            r#"
module example.com/test

go 1.21

require github.com/gin-gonic/gin v1.9.0
"#,
        )
        .unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert_eq!(project.primary_language, LanguageType::Go);
        assert!(project.build_systems.contains(&BuildSystem::GoModules));
        assert!(project.frameworks.contains(&FrameworkType::Gin));
    }

    #[test]
    fn test_detect_monorepo() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{ "workspaces": ["packages/*"] }"#,
        )
        .unwrap();
        fs::write(temp.path().join("turbo.json"), "{}").unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert!(project.is_monorepo);
        assert!(project.is_workspace);
    }

    #[test]
    fn test_confidence_calculation() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"
[package]
name = "test"

[dependencies]
axum = "0.7"
"#,
        )
        .unwrap();

        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        // Should have high confidence: language + build system + test framework + framework
        assert!(project.confidence >= 0.8);
    }

    #[test]
    fn test_empty_directory() {
        let temp = TempDir::new().unwrap();
        let detector = ProjectTypeDetector::new(temp.path());
        let project = detector.detect();

        assert_eq!(project.primary_language, LanguageType::Unknown);
        assert!(project.build_systems.is_empty());
        assert_eq!(project.confidence, 0.0);
    }
}
