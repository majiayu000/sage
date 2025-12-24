//! Project patterns and important file detection
//!
//! Identifies important files, entry points, and common project patterns.

mod language_patterns;
mod matcher;
mod types;

pub use matcher::PatternMatcher;
pub use types::{ImportantFile, ImportantFileType, ProjectPattern};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::detector::LanguageType;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_important_file_type_name() {
        assert_eq!(ImportantFileType::EntryPoint.name(), "Entry Point");
        assert_eq!(ImportantFileType::Config.name(), "Configuration");
    }

    #[test]
    fn test_pattern_builder() {
        let pattern = ProjectPattern::new("Test", ImportantFileType::Test)
            .with_patterns(["*.test.ts", "*.spec.ts"])
            .with_priority(80)
            .with_description("Test files");

        assert_eq!(pattern.name, "Test");
        assert_eq!(pattern.patterns.len(), 2);
        assert_eq!(pattern.priority, 80);
        assert_eq!(pattern.description.unwrap(), "Test files");
    }

    #[test]
    fn test_find_readme() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("README.md"), "# Test").unwrap();

        let matcher = PatternMatcher::for_language(LanguageType::Rust);
        let files = matcher.find_important_files(temp.path());

        assert!(!files.is_empty());
        let readme = files
            .iter()
            .find(|f| f.path.to_string_lossy().contains("README"));
        assert!(readme.is_some());
        assert_eq!(readme.unwrap().file_type, ImportantFileType::Documentation);
    }

    #[test]
    fn test_find_rust_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub fn lib() {}").unwrap();

        let matcher = PatternMatcher::for_language(LanguageType::Rust);
        let files = matcher.find_important_files(temp.path());

        let paths: Vec<_> = files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();

        assert!(paths.iter().any(|p| p.contains("Cargo.toml")));
        assert!(paths.iter().any(|p| p.contains("main.rs")));
        assert!(paths.iter().any(|p| p.contains("lib.rs")));
    }

    #[test]
    fn test_find_node_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        fs::write(temp.path().join("tsconfig.json"), "{}").unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/index.ts"), "").unwrap();

        let matcher = PatternMatcher::for_language(LanguageType::TypeScript);
        let files = matcher.find_important_files(temp.path());

        let paths: Vec<_> = files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();

        assert!(paths.iter().any(|p| p.contains("package.json")));
        assert!(paths.iter().any(|p| p.contains("tsconfig.json")));
        assert!(paths.iter().any(|p| p.contains("index.ts")));
    }

    #[test]
    fn test_priority_ordering() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(temp.path().join("README.md"), "# Test").unwrap();
        fs::write(temp.path().join(".gitignore"), "target/").unwrap();

        let matcher = PatternMatcher::for_language(LanguageType::Rust);
        let files = matcher.find_important_files(temp.path());

        // Cargo.toml should be first (highest priority)
        assert_eq!(files[0].path.to_string_lossy(), "Cargo.toml");
    }

    #[test]
    fn test_find_files_by_type() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(temp.path().join("README.md"), "# Test").unwrap();
        fs::write(temp.path().join("LICENSE"), "MIT").unwrap();

        let matcher = PatternMatcher::for_language(LanguageType::Rust);
        let by_type = matcher.find_files_by_type(temp.path());

        assert!(by_type.contains_key(&ImportantFileType::Build));
        assert!(by_type.contains_key(&ImportantFileType::Documentation));
        assert!(by_type.contains_key(&ImportantFileType::License));
    }

    #[test]
    fn test_empty_directory() {
        let temp = TempDir::new().unwrap();
        let matcher = PatternMatcher::for_language(LanguageType::Rust);
        let files = matcher.find_important_files(temp.path());

        assert!(files.is_empty());
    }

    #[test]
    fn test_cicd_detection() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".github/workflows")).unwrap();
        fs::write(temp.path().join(".github/workflows/ci.yml"), "name: CI").unwrap();

        let matcher = PatternMatcher::for_language(LanguageType::Rust);
        let files = matcher.find_important_files(temp.path());

        let cicd = files
            .iter()
            .find(|f| f.file_type == ImportantFileType::CiCd);
        assert!(cicd.is_some());
    }

    #[test]
    fn test_container_detection() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Dockerfile"), "FROM rust:1.70").unwrap();
        fs::write(temp.path().join("docker-compose.yml"), "version: '3'").unwrap();

        let matcher = PatternMatcher::for_language(LanguageType::Rust);
        let files = matcher.find_important_files(temp.path());

        let containers: Vec<_> = files
            .iter()
            .filter(|f| f.file_type == ImportantFileType::Container)
            .collect();
        assert_eq!(containers.len(), 2);
    }
}
