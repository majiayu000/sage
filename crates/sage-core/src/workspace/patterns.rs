//! Project patterns and important file detection
//!
//! Identifies important files, entry points, and common project patterns.

use super::detector::LanguageType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

/// Pattern matcher for finding important files
pub struct PatternMatcher {
    patterns: Vec<ProjectPattern>,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Create with default patterns for a language
    pub fn for_language(lang: LanguageType) -> Self {
        let mut matcher = Self::new();

        // Add universal patterns
        matcher.add_universal_patterns();

        // Add language-specific patterns
        match lang {
            LanguageType::Rust => matcher.add_rust_patterns(),
            LanguageType::TypeScript | LanguageType::JavaScript => matcher.add_node_patterns(),
            LanguageType::Python => matcher.add_python_patterns(),
            LanguageType::Go => matcher.add_go_patterns(),
            LanguageType::Java => matcher.add_java_patterns(),
            _ => {}
        }

        matcher
    }

    /// Add a pattern
    pub fn add_pattern(&mut self, pattern: ProjectPattern) {
        self.patterns.push(pattern);
    }

    fn add_universal_patterns(&mut self) {
        // Documentation
        self.add_pattern(
            ProjectPattern::new("README", ImportantFileType::Documentation)
                .with_patterns(["README.md", "README.txt", "README", "readme.md"])
                .with_priority(90)
                .with_description("Project documentation"),
        );

        self.add_pattern(
            ProjectPattern::new("License", ImportantFileType::License)
                .with_patterns(["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"])
                .with_priority(70),
        );

        self.add_pattern(
            ProjectPattern::new("Changelog", ImportantFileType::Documentation)
                .with_patterns(["CHANGELOG.md", "CHANGELOG", "HISTORY.md"])
                .with_priority(60),
        );

        self.add_pattern(
            ProjectPattern::new("Contributing", ImportantFileType::Documentation)
                .with_patterns(["CONTRIBUTING.md", "CONTRIBUTING"])
                .with_priority(50),
        );

        // CI/CD
        self.add_pattern(
            ProjectPattern::new("GitHub Actions", ImportantFileType::CiCd)
                .with_patterns([".github/workflows/*.yml", ".github/workflows/*.yaml"])
                .with_priority(70),
        );

        self.add_pattern(
            ProjectPattern::new("GitLab CI", ImportantFileType::CiCd)
                .with_patterns([".gitlab-ci.yml"])
                .with_priority(70),
        );

        self.add_pattern(
            ProjectPattern::new("CircleCI", ImportantFileType::CiCd)
                .with_patterns([".circleci/config.yml"])
                .with_priority(70),
        );

        // Container
        self.add_pattern(
            ProjectPattern::new("Dockerfile", ImportantFileType::Container)
                .with_patterns(["Dockerfile", "Dockerfile.*", "*.dockerfile"])
                .with_priority(75),
        );

        self.add_pattern(
            ProjectPattern::new("Docker Compose", ImportantFileType::Container)
                .with_patterns(["docker-compose.yml", "docker-compose.yaml", "compose.yml"])
                .with_priority(75),
        );

        // Environment
        self.add_pattern(
            ProjectPattern::new("Environment", ImportantFileType::Environment)
                .with_patterns([".env", ".env.example", ".env.local", ".env.development"])
                .with_priority(80),
        );

        // Git
        self.add_pattern(
            ProjectPattern::new("Gitignore", ImportantFileType::Config)
                .with_patterns([".gitignore"])
                .with_priority(50),
        );

        // Editor config
        self.add_pattern(
            ProjectPattern::new("EditorConfig", ImportantFileType::Config)
                .with_patterns([".editorconfig"])
                .with_priority(30),
        );
    }

    fn add_rust_patterns(&mut self) {
        // Entry points
        self.add_pattern(
            ProjectPattern::new("Main", ImportantFileType::EntryPoint)
                .with_patterns(["src/main.rs", "src/bin/*.rs"])
                .with_priority(100)
                .with_description("Application entry point"),
        );

        self.add_pattern(
            ProjectPattern::new("Library", ImportantFileType::EntryPoint)
                .with_patterns(["src/lib.rs"])
                .with_priority(95)
                .with_description("Library entry point"),
        );

        // Build
        self.add_pattern(
            ProjectPattern::new("Cargo.toml", ImportantFileType::Build)
                .with_patterns(["Cargo.toml"])
                .with_priority(100)
                .with_description("Cargo manifest"),
        );

        self.add_pattern(
            ProjectPattern::new("Cargo.lock", ImportantFileType::LockFile)
                .with_patterns(["Cargo.lock"])
                .with_priority(60),
        );

        // Config
        self.add_pattern(
            ProjectPattern::new("Rust Config", ImportantFileType::Config)
                .with_patterns([
                    "rust-toolchain.toml",
                    "rustfmt.toml",
                    ".rustfmt.toml",
                    "clippy.toml",
                ])
                .with_priority(60),
        );

        // Tests
        self.add_pattern(
            ProjectPattern::new("Tests", ImportantFileType::Test)
                .with_patterns(["tests/*.rs", "tests/**/*.rs"])
                .with_priority(70),
        );

        // Build script
        self.add_pattern(
            ProjectPattern::new("Build Script", ImportantFileType::Build)
                .with_patterns(["build.rs"])
                .with_priority(80),
        );
    }

    fn add_node_patterns(&mut self) {
        // Entry points
        self.add_pattern(
            ProjectPattern::new("Index", ImportantFileType::EntryPoint)
                .with_patterns([
                    "src/index.ts",
                    "src/index.tsx",
                    "src/index.js",
                    "src/index.jsx",
                    "index.ts",
                    "index.js",
                ])
                .with_priority(95),
        );

        self.add_pattern(
            ProjectPattern::new("App", ImportantFileType::EntryPoint)
                .with_patterns([
                    "src/App.tsx",
                    "src/App.jsx",
                    "src/app.ts",
                    "src/app.js",
                    "app/page.tsx",
                    "app/page.jsx", // Next.js app router
                    "pages/_app.tsx",
                    "pages/_app.jsx", // Next.js pages router
                ])
                .with_priority(90),
        );

        self.add_pattern(
            ProjectPattern::new("Server", ImportantFileType::EntryPoint)
                .with_patterns([
                    "server.ts",
                    "server.js",
                    "src/server.ts",
                    "src/server.js",
                    "src/main.ts",
                    "src/main.js",
                ])
                .with_priority(90),
        );

        // Build
        self.add_pattern(
            ProjectPattern::new("Package.json", ImportantFileType::Build)
                .with_patterns(["package.json"])
                .with_priority(100),
        );

        // Lock files
        self.add_pattern(
            ProjectPattern::new("Lock File", ImportantFileType::LockFile)
                .with_patterns([
                    "package-lock.json",
                    "yarn.lock",
                    "pnpm-lock.yaml",
                    "bun.lockb",
                ])
                .with_priority(60),
        );

        // Config
        self.add_pattern(
            ProjectPattern::new("TypeScript Config", ImportantFileType::Config)
                .with_patterns(["tsconfig.json", "tsconfig.*.json"])
                .with_priority(85),
        );

        self.add_pattern(
            ProjectPattern::new("ESLint", ImportantFileType::Config)
                .with_patterns([
                    ".eslintrc",
                    ".eslintrc.js",
                    ".eslintrc.json",
                    ".eslintrc.cjs",
                    "eslint.config.js",
                    "eslint.config.mjs",
                ])
                .with_priority(60),
        );

        self.add_pattern(
            ProjectPattern::new("Prettier", ImportantFileType::Config)
                .with_patterns([
                    ".prettierrc",
                    ".prettierrc.js",
                    ".prettierrc.json",
                    "prettier.config.js",
                    "prettier.config.mjs",
                ])
                .with_priority(50),
        );

        self.add_pattern(
            ProjectPattern::new("Vite Config", ImportantFileType::Build)
                .with_patterns(["vite.config.ts", "vite.config.js"])
                .with_priority(80),
        );

        self.add_pattern(
            ProjectPattern::new("Next.js Config", ImportantFileType::Build)
                .with_patterns(["next.config.js", "next.config.mjs", "next.config.ts"])
                .with_priority(85),
        );

        self.add_pattern(
            ProjectPattern::new("Webpack Config", ImportantFileType::Build)
                .with_patterns(["webpack.config.js", "webpack.config.ts"])
                .with_priority(80),
        );

        // Tests
        self.add_pattern(
            ProjectPattern::new("Test Config", ImportantFileType::Config)
                .with_patterns([
                    "jest.config.js",
                    "jest.config.ts",
                    "vitest.config.ts",
                    "playwright.config.ts",
                    "cypress.config.ts",
                ])
                .with_priority(70),
        );

        // Type definitions
        self.add_pattern(
            ProjectPattern::new("Type Definitions", ImportantFileType::TypeDefinition)
                .with_patterns(["*.d.ts", "types/*.ts", "typings/*.ts"])
                .with_priority(65),
        );

        // API
        self.add_pattern(
            ProjectPattern::new("OpenAPI", ImportantFileType::ApiDefinition)
                .with_patterns([
                    "openapi.yaml",
                    "openapi.json",
                    "swagger.yaml",
                    "swagger.json",
                ])
                .with_priority(75),
        );
    }

    fn add_python_patterns(&mut self) {
        // Entry points
        self.add_pattern(
            ProjectPattern::new("Main", ImportantFileType::EntryPoint)
                .with_patterns(["main.py", "app.py", "run.py", "__main__.py"])
                .with_priority(95),
        );

        self.add_pattern(
            ProjectPattern::new("Package Init", ImportantFileType::EntryPoint)
                .with_patterns(["src/*/__init__.py", "*/__init__.py"])
                .with_priority(80),
        );

        // Build
        self.add_pattern(
            ProjectPattern::new("Pyproject", ImportantFileType::Build)
                .with_patterns(["pyproject.toml"])
                .with_priority(100),
        );

        self.add_pattern(
            ProjectPattern::new("Setup", ImportantFileType::Build)
                .with_patterns(["setup.py", "setup.cfg"])
                .with_priority(90),
        );

        self.add_pattern(
            ProjectPattern::new("Requirements", ImportantFileType::Build)
                .with_patterns(["requirements.txt", "requirements/*.txt"])
                .with_priority(85),
        );

        // Lock files
        self.add_pattern(
            ProjectPattern::new("Lock File", ImportantFileType::LockFile)
                .with_patterns(["poetry.lock", "Pipfile.lock", "pdm.lock", "uv.lock"])
                .with_priority(60),
        );

        // Config
        self.add_pattern(
            ProjectPattern::new("Pytest Config", ImportantFileType::Config)
                .with_patterns(["pytest.ini", "pyproject.toml", "conftest.py"])
                .with_priority(70),
        );

        self.add_pattern(
            ProjectPattern::new("Linter Config", ImportantFileType::Config)
                .with_patterns([".flake8", "ruff.toml", ".ruff.toml", ".pylintrc"])
                .with_priority(50),
        );

        // Tests
        self.add_pattern(
            ProjectPattern::new("Tests", ImportantFileType::Test)
                .with_patterns(["tests/*.py", "test_*.py", "*_test.py"])
                .with_priority(70),
        );

        // Type definitions
        self.add_pattern(
            ProjectPattern::new("Type Stubs", ImportantFileType::TypeDefinition)
                .with_patterns(["*.pyi", "py.typed"])
                .with_priority(60),
        );

        // Database
        self.add_pattern(
            ProjectPattern::new("Alembic", ImportantFileType::Database)
                .with_patterns(["alembic.ini", "alembic/*.py"])
                .with_priority(70),
        );
    }

    fn add_go_patterns(&mut self) {
        // Entry points
        self.add_pattern(
            ProjectPattern::new("Main", ImportantFileType::EntryPoint)
                .with_patterns(["main.go", "cmd/*/main.go"])
                .with_priority(100),
        );

        // Build
        self.add_pattern(
            ProjectPattern::new("Go Mod", ImportantFileType::Build)
                .with_patterns(["go.mod"])
                .with_priority(100),
        );

        self.add_pattern(
            ProjectPattern::new("Go Sum", ImportantFileType::LockFile)
                .with_patterns(["go.sum"])
                .with_priority(60),
        );

        self.add_pattern(
            ProjectPattern::new("Makefile", ImportantFileType::Build)
                .with_patterns(["Makefile"])
                .with_priority(80),
        );

        // Tests
        self.add_pattern(
            ProjectPattern::new("Tests", ImportantFileType::Test)
                .with_patterns(["*_test.go", "**/*_test.go"])
                .with_priority(70),
        );

        // Config
        self.add_pattern(
            ProjectPattern::new("Golangci", ImportantFileType::Config)
                .with_patterns([".golangci.yml", ".golangci.yaml"])
                .with_priority(60),
        );
    }

    fn add_java_patterns(&mut self) {
        // Entry points
        self.add_pattern(
            ProjectPattern::new("Main", ImportantFileType::EntryPoint)
                .with_patterns([
                    "**/Application.java",
                    "**/Main.java",
                    "**/App.java",
                    "**/Application.kt",
                    "**/Main.kt",
                ])
                .with_priority(95),
        );

        // Build
        self.add_pattern(
            ProjectPattern::new("Maven", ImportantFileType::Build)
                .with_patterns(["pom.xml"])
                .with_priority(100),
        );

        self.add_pattern(
            ProjectPattern::new("Gradle", ImportantFileType::Build)
                .with_patterns([
                    "build.gradle",
                    "build.gradle.kts",
                    "settings.gradle",
                    "settings.gradle.kts",
                ])
                .with_priority(100),
        );

        // Config
        self.add_pattern(
            ProjectPattern::new("Application Config", ImportantFileType::Config)
                .with_patterns([
                    "src/main/resources/application.properties",
                    "src/main/resources/application.yml",
                    "src/main/resources/application.yaml",
                ])
                .with_priority(90),
        );

        // Tests
        self.add_pattern(
            ProjectPattern::new("Tests", ImportantFileType::Test)
                .with_patterns(["src/test/java/**/*.java", "src/test/kotlin/**/*.kt"])
                .with_priority(70),
        );
    }

    /// Find important files in a directory
    pub fn find_important_files(&self, root: &Path) -> Vec<ImportantFile> {
        let mut files = Vec::new();

        for pattern in &self.patterns {
            for glob_pattern in &pattern.patterns {
                let full_pattern = root.join(glob_pattern);
                if let Ok(entries) = glob::glob(full_pattern.to_string_lossy().as_ref()) {
                    for entry in entries.flatten() {
                        if entry.is_file() {
                            let relative = entry.strip_prefix(root).unwrap_or(&entry).to_path_buf();
                            files.push(ImportantFile {
                                path: relative,
                                file_type: pattern.file_type,
                                description: pattern.description.clone(),
                                priority: pattern.priority,
                            });
                        }
                    }
                }
            }
        }

        // Check for exact matches in root
        for pattern in &self.patterns {
            for glob_pattern in &pattern.patterns {
                if !glob_pattern.contains('/') && !glob_pattern.contains('*') {
                    let path = root.join(glob_pattern);
                    if path.exists() && path.is_file() {
                        let exists = files.iter().any(|f| f.path == PathBuf::from(glob_pattern));
                        if !exists {
                            files.push(ImportantFile {
                                path: PathBuf::from(glob_pattern),
                                file_type: pattern.file_type,
                                description: pattern.description.clone(),
                                priority: pattern.priority,
                            });
                        }
                    }
                }
            }
        }

        // Sort by priority (descending) then by path
        files.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.path.cmp(&b.path))
        });

        // Deduplicate by path
        let mut seen = std::collections::HashSet::new();
        files.retain(|f| seen.insert(f.path.clone()));

        files
    }

    /// Get files grouped by type
    pub fn find_files_by_type(
        &self,
        root: &Path,
    ) -> HashMap<ImportantFileType, Vec<ImportantFile>> {
        let files = self.find_important_files(root);
        let mut by_type: HashMap<ImportantFileType, Vec<ImportantFile>> = HashMap::new();

        for file in files {
            by_type.entry(file.file_type).or_default().push(file);
        }

        by_type
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
