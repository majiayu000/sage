//! Project structure analysis

use std::path::{Path, PathBuf};

use super::detector::{LanguageType, ProjectType};
use super::models::ProjectStructure;

/// Analyze project structure
pub fn analyze_structure(root: &Path, project: &ProjectType) -> ProjectStructure {
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
        LanguageType::TypeScript | LanguageType::JavaScript => {
            vec!["src", "lib", "app", "pages", "components"]
        }
        LanguageType::Python => vec!["src", "lib"],
        LanguageType::Go => vec!["cmd", "pkg", "internal"],
        LanguageType::Java => vec!["src/main/java", "src/main/kotlin"],
        _ => vec!["src", "lib"],
    };

    for candidate in source_candidates {
        let path = root.join(candidate);
        if path.exists() && path.is_dir() {
            structure.source_dirs.push(PathBuf::from(candidate));
        }
    }

    // Test directories
    let test_candidates = match project.primary_language {
        LanguageType::Rust => vec!["tests"],
        LanguageType::TypeScript | LanguageType::JavaScript => {
            vec!["tests", "test", "__tests__", "spec"]
        }
        LanguageType::Python => vec!["tests", "test"],
        LanguageType::Go => vec![], // Go tests are in same dir
        LanguageType::Java => vec!["src/test/java", "src/test/kotlin"],
        _ => vec!["tests", "test"],
    };

    for candidate in test_candidates {
        let path = root.join(candidate);
        if path.exists() && path.is_dir() {
            structure.test_dirs.push(PathBuf::from(candidate));
        }
    }

    // Documentation directories
    for candidate in ["docs", "doc", "documentation"] {
        let path = root.join(candidate);
        if path.exists() && path.is_dir() {
            structure.doc_dirs.push(PathBuf::from(candidate));
        }
    }

    // Build directories (usually gitignored)
    let build_candidates = match project.primary_language {
        LanguageType::Rust => vec!["target"],
        LanguageType::TypeScript | LanguageType::JavaScript => {
            vec!["dist", "build", ".next", ".nuxt", "out"]
        }
        LanguageType::Python => vec!["dist", "build", "__pycache__"],
        LanguageType::Go => vec!["bin"],
        LanguageType::Java => vec!["target", "build", "out"],
        _ => vec!["dist", "build"],
    };

    for candidate in build_candidates {
        let path = root.join(candidate);
        if path.exists() && path.is_dir() {
            structure.build_dirs.push(PathBuf::from(candidate));
        }
    }

    // Config directory
    for candidate in [".config", "config", ".sage", ".claude"] {
        let path = root.join(candidate);
        if path.exists() && path.is_dir() {
            structure.config_dir = Some(PathBuf::from(candidate));
            break;
        }
    }

    // Check if conventional structure
    structure.is_conventional = !structure.source_dirs.is_empty();

    structure
}
