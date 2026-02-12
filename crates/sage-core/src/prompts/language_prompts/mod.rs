//! Language-specialized prompts
//!
//! This module provides language-specific guidance and best practices,
//! following Claude Code's pattern of context-aware assistance.
//!
//! # Supported Languages
//!
//! - Rust: Memory safety, ownership, error handling
//! - Python: Type hints, async patterns, packaging
//! - TypeScript: Type safety, React patterns, Node.js
//! - Go: Error handling, concurrency, interfaces
//! - Java: OOP patterns, Spring conventions
//! - C/C++: Memory management, RAII, modern C++
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::prompts::{Language, LanguagePrompts};
//!
//! let lang = Language::detect_from_extension("rs");
//! let guidance = LanguagePrompts::for_language(lang);
//! ```

mod guidance;
mod language;

pub use guidance::LanguagePrompts;
pub use language::{Language, detect_primary_language};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
        assert_eq!(Language::from_extension("go"), Language::Go);
        assert_eq!(Language::from_extension("java"), Language::Java);
        assert_eq!(Language::from_extension("unknown"), Language::Unknown);
    }

    #[test]
    fn test_language_from_path() {
        assert_eq!(
            Language::from_path(Path::new("src/main.rs")),
            Language::Rust
        );
        assert_eq!(
            Language::from_path(Path::new("app/views/index.tsx")),
            Language::TypeScript
        );
        assert_eq!(
            Language::from_path(Path::new("no_extension")),
            Language::Unknown
        );
    }

    #[test]
    fn test_language_name() {
        assert_eq!(Language::Rust.name(), "Rust");
        assert_eq!(Language::TypeScript.name(), "TypeScript");
        assert_eq!(Language::CSharp.name(), "C#");
    }

    #[test]
    fn test_is_systems_language() {
        assert!(Language::Rust.is_systems_language());
        assert!(Language::C.is_systems_language());
        assert!(Language::Cpp.is_systems_language());
        assert!(Language::Go.is_systems_language());
        assert!(!Language::Python.is_systems_language());
        assert!(!Language::JavaScript.is_systems_language());
    }

    #[test]
    fn test_is_scripting_language() {
        assert!(Language::Python.is_scripting_language());
        assert!(Language::JavaScript.is_scripting_language());
        assert!(Language::Ruby.is_scripting_language());
        assert!(!Language::Rust.is_scripting_language());
        assert!(!Language::Java.is_scripting_language());
    }

    #[test]
    fn test_is_markup_or_config() {
        assert!(Language::Html.is_markup_or_config());
        assert!(Language::Json.is_markup_or_config());
        assert!(Language::Yaml.is_markup_or_config());
        assert!(!Language::Rust.is_markup_or_config());
        assert!(!Language::Python.is_markup_or_config());
    }

    #[test]
    fn test_language_prompts_exist() {
        // Verify all major languages have prompts
        let languages = [
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::JavaScript,
            Language::Go,
            Language::Java,
            Language::CSharp,
            Language::Cpp,
            Language::C,
            Language::Ruby,
            Language::Swift,
            Language::Kotlin,
            Language::Shell,
        ];

        for lang in languages {
            let prompt = LanguagePrompts::for_language(lang);
            assert!(!prompt.is_empty());
            assert!(prompt.contains("##"));
        }
    }

    #[test]
    fn test_compact_hints() {
        let hint = LanguagePrompts::compact_hint(Language::Rust);
        assert!(hint.contains("Rust"));
        assert!(hint.contains("Result"));

        let hint = LanguagePrompts::compact_hint(Language::Python);
        assert!(hint.contains("Python"));
        assert!(hint.contains("type hints"));
    }

    #[test]
    fn test_detect_primary_language() {
        let paths: Vec<&Path> = vec![
            Path::new("src/main.rs"),
            Path::new("src/lib.rs"),
            Path::new("src/utils.rs"),
            Path::new("Cargo.toml"),
            Path::new("README.md"),
        ];

        assert_eq!(detect_primary_language(&paths), Language::Rust);
    }

    #[test]
    fn test_detect_primary_language_mixed() {
        let paths: Vec<&Path> = vec![
            Path::new("src/main.py"),
            Path::new("src/utils.py"),
            Path::new("tests/test_main.py"),
            Path::new("src/helper.js"),
            Path::new("package.json"),
        ];

        assert_eq!(detect_primary_language(&paths), Language::Python);
    }

    #[test]
    fn test_detect_primary_language_empty() {
        let paths: Vec<&Path> = vec![];
        assert_eq!(detect_primary_language(&paths), Language::Unknown);
    }
}
