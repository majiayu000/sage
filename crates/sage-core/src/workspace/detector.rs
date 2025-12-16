//! Project type detection
//!
//! Detects programming languages, frameworks, build systems, and test frameworks
//! based on project files and structure.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Primary programming language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageType {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Ruby,
    Swift,
    Kotlin,
    Scala,
    Php,
    Shell,
    Unknown,
}

impl LanguageType {
    /// Get file extensions for this language
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Rust => &["rs"],
            Self::TypeScript => &["ts", "tsx"],
            Self::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Self::Python => &["py", "pyi"],
            Self::Go => &["go"],
            Self::Java => &["java"],
            Self::CSharp => &["cs"],
            Self::Cpp => &["cpp", "cc", "cxx", "hpp", "hh"],
            Self::C => &["c", "h"],
            Self::Ruby => &["rb"],
            Self::Swift => &["swift"],
            Self::Kotlin => &["kt", "kts"],
            Self::Scala => &["scala", "sc"],
            Self::Php => &["php"],
            Self::Shell => &["sh", "bash", "zsh"],
            Self::Unknown => &[],
        }
    }

    /// Get the language name
    pub fn name(&self) -> &str {
        match self {
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
            Self::JavaScript => "JavaScript",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::CSharp => "C#",
            Self::Cpp => "C++",
            Self::C => "C",
            Self::Ruby => "Ruby",
            Self::Swift => "Swift",
            Self::Kotlin => "Kotlin",
            Self::Scala => "Scala",
            Self::Php => "PHP",
            Self::Shell => "Shell",
            Self::Unknown => "Unknown",
        }
    }
}

/// Framework type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameworkType {
    // Web frameworks
    React,
    Vue,
    Angular,
    Svelte,
    NextJs,
    Nuxt,
    Express,
    Fastify,
    NestJs,
    Django,
    Flask,
    FastApi,
    Rails,
    Spring,
    Actix,
    Axum,
    Rocket,
    Gin,
    Echo,
    // Mobile
    ReactNative,
    Flutter,
    SwiftUI,
    Jetpack,
    // Other
    Electron,
    Tauri,
    Custom(String),
}

impl FrameworkType {
    /// Get the framework name
    pub fn name(&self) -> &str {
        match self {
            Self::React => "React",
            Self::Vue => "Vue",
            Self::Angular => "Angular",
            Self::Svelte => "Svelte",
            Self::NextJs => "Next.js",
            Self::Nuxt => "Nuxt",
            Self::Express => "Express",
            Self::Fastify => "Fastify",
            Self::NestJs => "NestJS",
            Self::Django => "Django",
            Self::Flask => "Flask",
            Self::FastApi => "FastAPI",
            Self::Rails => "Rails",
            Self::Spring => "Spring",
            Self::Actix => "Actix",
            Self::Axum => "Axum",
            Self::Rocket => "Rocket",
            Self::Gin => "Gin",
            Self::Echo => "Echo",
            Self::ReactNative => "React Native",
            Self::Flutter => "Flutter",
            Self::SwiftUI => "SwiftUI",
            Self::Jetpack => "Jetpack Compose",
            Self::Electron => "Electron",
            Self::Tauri => "Tauri",
            Self::Custom(name) => name,
        }
    }
}

/// Build system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildSystem {
    Cargo,
    Npm,
    Yarn,
    Pnpm,
    Bun,
    Pip,
    Poetry,
    Pdm,
    Uv,
    Maven,
    Gradle,
    Sbt,
    Mix,
    GoModules,
    CMake,
    Make,
    Bazel,
    Meson,
    Custom(String),
}

impl BuildSystem {
    /// Get the build system name
    pub fn name(&self) -> &str {
        match self {
            Self::Cargo => "Cargo",
            Self::Npm => "npm",
            Self::Yarn => "Yarn",
            Self::Pnpm => "pnpm",
            Self::Bun => "Bun",
            Self::Pip => "pip",
            Self::Poetry => "Poetry",
            Self::Pdm => "PDM",
            Self::Uv => "uv",
            Self::Maven => "Maven",
            Self::Gradle => "Gradle",
            Self::Sbt => "sbt",
            Self::Mix => "Mix",
            Self::GoModules => "Go Modules",
            Self::CMake => "CMake",
            Self::Make => "Make",
            Self::Bazel => "Bazel",
            Self::Meson => "Meson",
            Self::Custom(name) => name,
        }
    }

    /// Get the config file name
    pub fn config_file(&self) -> Option<&str> {
        match self {
            Self::Cargo => Some("Cargo.toml"),
            Self::Npm | Self::Yarn | Self::Pnpm | Self::Bun => Some("package.json"),
            Self::Poetry => Some("pyproject.toml"),
            Self::Pdm => Some("pyproject.toml"),
            Self::Uv => Some("pyproject.toml"),
            Self::Maven => Some("pom.xml"),
            Self::Gradle => Some("build.gradle"),
            Self::Sbt => Some("build.sbt"),
            Self::Mix => Some("mix.exs"),
            Self::GoModules => Some("go.mod"),
            Self::CMake => Some("CMakeLists.txt"),
            Self::Make => Some("Makefile"),
            Self::Bazel => Some("BUILD"),
            Self::Meson => Some("meson.build"),
            _ => None,
        }
    }
}

/// Test framework
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestFramework {
    // Rust
    RustBuiltin,
    // JavaScript/TypeScript
    Jest,
    Vitest,
    Mocha,
    Playwright,
    Cypress,
    // Python
    Pytest,
    Unittest,
    // Go
    GoTest,
    // Java
    JUnit,
    TestNG,
    // Other
    RSpec,
    PHPUnit,
    Custom(String),
}

impl TestFramework {
    /// Get the test framework name
    pub fn name(&self) -> &str {
        match self {
            Self::RustBuiltin => "Rust built-in",
            Self::Jest => "Jest",
            Self::Vitest => "Vitest",
            Self::Mocha => "Mocha",
            Self::Playwright => "Playwright",
            Self::Cypress => "Cypress",
            Self::Pytest => "pytest",
            Self::Unittest => "unittest",
            Self::GoTest => "go test",
            Self::JUnit => "JUnit",
            Self::TestNG => "TestNG",
            Self::RSpec => "RSpec",
            Self::PHPUnit => "PHPUnit",
            Self::Custom(name) => name,
        }
    }
}

/// Runtime type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    Node,
    Deno,
    Bun,
    Python,
    Jvm,
    DotNet,
    Native,
    Wasm,
    Browser,
    Custom(String),
}

impl RuntimeType {
    /// Get the runtime name
    pub fn name(&self) -> &str {
        match self {
            Self::Node => "Node.js",
            Self::Deno => "Deno",
            Self::Bun => "Bun",
            Self::Python => "Python",
            Self::Jvm => "JVM",
            Self::DotNet => ".NET",
            Self::Native => "Native",
            Self::Wasm => "WebAssembly",
            Self::Browser => "Browser",
            Self::Custom(name) => name,
        }
    }
}

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

        // Check for various config files
        self.detect_rust(&mut project_type);
        self.detect_node(&mut project_type);
        self.detect_python(&mut project_type);
        self.detect_go(&mut project_type);
        self.detect_java(&mut project_type);
        self.detect_other(&mut project_type);

        // Determine primary language if not set
        if project_type.primary_language == LanguageType::Unknown {
            project_type.primary_language = self.detect_by_file_count();
        }

        // Calculate confidence
        project_type.confidence = self.calculate_confidence(&project_type);

        project_type
    }

    fn detect_rust(&self, project: &mut ProjectType) {
        let cargo_toml = self.root.join("Cargo.toml");
        if cargo_toml.exists() {
            project.primary_language = LanguageType::Rust;
            project.build_systems.insert(BuildSystem::Cargo);
            project.test_frameworks.insert(TestFramework::RustBuiltin);
            project.runtime = Some(RuntimeType::Native);

            // Check for workspace
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    project.is_workspace = true;
                }

                // Detect frameworks
                if content.contains("actix") {
                    project.frameworks.insert(FrameworkType::Actix);
                }
                if content.contains("axum") {
                    project.frameworks.insert(FrameworkType::Axum);
                }
                if content.contains("rocket") {
                    project.frameworks.insert(FrameworkType::Rocket);
                }
                if content.contains("tauri") {
                    project.frameworks.insert(FrameworkType::Tauri);
                }
            }
        }
    }

    fn detect_node(&self, project: &mut ProjectType) {
        let package_json = self.root.join("package.json");
        if package_json.exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::JavaScript;
            }
            project.runtime = Some(RuntimeType::Node);

            // Detect package manager
            if self.root.join("yarn.lock").exists() {
                project.build_systems.insert(BuildSystem::Yarn);
            } else if self.root.join("pnpm-lock.yaml").exists() {
                project.build_systems.insert(BuildSystem::Pnpm);
            } else if self.root.join("bun.lockb").exists() {
                project.build_systems.insert(BuildSystem::Bun);
                project.runtime = Some(RuntimeType::Bun);
            } else {
                project.build_systems.insert(BuildSystem::Npm);
            }

            // Check for TypeScript
            if self.root.join("tsconfig.json").exists() {
                project.primary_language = LanguageType::TypeScript;
                project.secondary_languages.insert(LanguageType::JavaScript);
            }

            // Parse package.json for frameworks
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                self.detect_node_frameworks(&content, project);
                self.detect_node_test_frameworks(&content, project);

                // Check for workspaces
                if content.contains("\"workspaces\"") {
                    project.is_workspace = true;
                    project.is_monorepo = true;
                }
            }
        }

        // Check for Deno
        if self.root.join("deno.json").exists() || self.root.join("deno.jsonc").exists() {
            project.runtime = Some(RuntimeType::Deno);
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::TypeScript;
            }
        }
    }

    fn detect_node_frameworks(&self, content: &str, project: &mut ProjectType) {
        let checks = [
            ("react", FrameworkType::React),
            ("vue", FrameworkType::Vue),
            ("@angular/core", FrameworkType::Angular),
            ("svelte", FrameworkType::Svelte),
            ("next", FrameworkType::NextJs),
            ("nuxt", FrameworkType::Nuxt),
            ("express", FrameworkType::Express),
            ("fastify", FrameworkType::Fastify),
            ("@nestjs/core", FrameworkType::NestJs),
            ("electron", FrameworkType::Electron),
            ("react-native", FrameworkType::ReactNative),
        ];

        for (marker, framework) in checks {
            if content.contains(&format!("\"{}\"", marker)) {
                project.frameworks.insert(framework);
            }
        }
    }

    fn detect_node_test_frameworks(&self, content: &str, project: &mut ProjectType) {
        if content.contains("\"jest\"") {
            project.test_frameworks.insert(TestFramework::Jest);
        }
        if content.contains("\"vitest\"") {
            project.test_frameworks.insert(TestFramework::Vitest);
        }
        if content.contains("\"mocha\"") {
            project.test_frameworks.insert(TestFramework::Mocha);
        }
        if content.contains("\"playwright\"") || content.contains("\"@playwright/test\"") {
            project.test_frameworks.insert(TestFramework::Playwright);
        }
        if content.contains("\"cypress\"") {
            project.test_frameworks.insert(TestFramework::Cypress);
        }
    }

    fn detect_python(&self, project: &mut ProjectType) {
        // Check for pyproject.toml
        let pyproject = self.root.join("pyproject.toml");
        if pyproject.exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Python;
            }
            project.runtime = Some(RuntimeType::Python);

            if let Ok(content) = std::fs::read_to_string(&pyproject) {
                // Detect build system
                if content.contains("[tool.poetry]") {
                    project.build_systems.insert(BuildSystem::Poetry);
                } else if content.contains("[tool.pdm]") {
                    project.build_systems.insert(BuildSystem::Pdm);
                } else if content.contains("[tool.uv]")
                    || self.root.join("uv.lock").exists()
                {
                    project.build_systems.insert(BuildSystem::Uv);
                } else {
                    project.build_systems.insert(BuildSystem::Pip);
                }

                // Detect frameworks
                if content.contains("django") {
                    project.frameworks.insert(FrameworkType::Django);
                }
                if content.contains("flask") {
                    project.frameworks.insert(FrameworkType::Flask);
                }
                if content.contains("fastapi") {
                    project.frameworks.insert(FrameworkType::FastApi);
                }

                // Detect test frameworks
                if content.contains("pytest") {
                    project.test_frameworks.insert(TestFramework::Pytest);
                }
            }
        }

        // Check for requirements.txt
        if self.root.join("requirements.txt").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Python;
            }
            project.build_systems.insert(BuildSystem::Pip);
            project.runtime = Some(RuntimeType::Python);
        }

        // Check for setup.py
        if self.root.join("setup.py").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Python;
            }
            project.build_systems.insert(BuildSystem::Pip);
        }

        // Check for pytest
        if self.root.join("pytest.ini").exists()
            || self.root.join("conftest.py").exists()
        {
            project.test_frameworks.insert(TestFramework::Pytest);
        }
    }

    fn detect_go(&self, project: &mut ProjectType) {
        if self.root.join("go.mod").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Go;
            }
            project.build_systems.insert(BuildSystem::GoModules);
            project.test_frameworks.insert(TestFramework::GoTest);
            project.runtime = Some(RuntimeType::Native);

            // Detect frameworks from go.mod
            if let Ok(content) = std::fs::read_to_string(self.root.join("go.mod")) {
                if content.contains("gin-gonic/gin") {
                    project.frameworks.insert(FrameworkType::Gin);
                }
                if content.contains("labstack/echo") {
                    project.frameworks.insert(FrameworkType::Echo);
                }
            }
        }
    }

    fn detect_java(&self, project: &mut ProjectType) {
        // Maven
        if self.root.join("pom.xml").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Java;
            }
            project.build_systems.insert(BuildSystem::Maven);
            project.runtime = Some(RuntimeType::Jvm);
        }

        // Gradle
        if self.root.join("build.gradle").exists()
            || self.root.join("build.gradle.kts").exists()
        {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Java;
            }
            project.build_systems.insert(BuildSystem::Gradle);
            project.runtime = Some(RuntimeType::Jvm);

            // Check for Kotlin
            if self.root.join("build.gradle.kts").exists() {
                project.secondary_languages.insert(LanguageType::Kotlin);
            }
        }

        // sbt (Scala)
        if self.root.join("build.sbt").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Scala;
            }
            project.build_systems.insert(BuildSystem::Sbt);
            project.runtime = Some(RuntimeType::Jvm);
        }
    }

    fn detect_other(&self, project: &mut ProjectType) {
        // Ruby on Rails
        if self.root.join("Gemfile").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Ruby;
            }

            if let Ok(content) = std::fs::read_to_string(self.root.join("Gemfile")) {
                if content.contains("rails") {
                    project.frameworks.insert(FrameworkType::Rails);
                }
                if content.contains("rspec") {
                    project.test_frameworks.insert(TestFramework::RSpec);
                }
            }
        }

        // CMake
        if self.root.join("CMakeLists.txt").exists() {
            project.build_systems.insert(BuildSystem::CMake);
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Cpp;
            }
            project.runtime = Some(RuntimeType::Native);
        }

        // Make
        if self.root.join("Makefile").exists() {
            project.build_systems.insert(BuildSystem::Make);
        }

        // C#/.NET
        if self.root.join("*.csproj").exists() || self.root.join("*.sln").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::CSharp;
            }
            project.runtime = Some(RuntimeType::DotNet);
        }

        // PHP
        if self.root.join("composer.json").exists() {
            if project.primary_language == LanguageType::Unknown {
                project.primary_language = LanguageType::Php;
            }

            if let Ok(content) = std::fs::read_to_string(self.root.join("composer.json")) {
                if content.contains("phpunit") {
                    project.test_frameworks.insert(TestFramework::PHPUnit);
                }
            }
        }

        // Check for monorepo indicators
        if self.root.join("lerna.json").exists()
            || self.root.join("nx.json").exists()
            || self.root.join("turbo.json").exists()
            || self.root.join("rush.json").exists()
        {
            project.is_monorepo = true;
        }
    }

    fn detect_by_file_count(&self) -> LanguageType {
        let mut counts: std::collections::HashMap<LanguageType, usize> = std::collections::HashMap::new();

        let languages = [
            LanguageType::Rust,
            LanguageType::TypeScript,
            LanguageType::JavaScript,
            LanguageType::Python,
            LanguageType::Go,
            LanguageType::Java,
            LanguageType::CSharp,
            LanguageType::Cpp,
            LanguageType::Ruby,
        ];

        for lang in languages {
            let count = self.count_files_with_extensions(lang.extensions());
            if count > 0 {
                counts.insert(lang, count);
            }
        }

        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
            .unwrap_or(LanguageType::Unknown)
    }

    fn count_files_with_extensions(&self, extensions: &[&str]) -> usize {
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if extensions.contains(&ext.to_string_lossy().as_ref()) {
                            count += 1;
                        }
                    }
                } else if path.is_dir() && self.max_depth > 0 {
                    // Simple recursive count (limited depth)
                    let detector = ProjectTypeDetector::new(&path)
                        .with_max_depth(self.max_depth - 1);
                    count += detector.count_files_with_extensions(extensions);
                }
            }
        }
        count
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
        assert!(project.test_frameworks.contains(&TestFramework::RustBuiltin));
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
