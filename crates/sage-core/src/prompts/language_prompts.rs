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

use std::path::Path;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Ruby,
    Swift,
    Kotlin,
    Php,
    Shell,
    Sql,
    Html,
    Css,
    Markdown,
    Json,
    Yaml,
    Toml,
    Unknown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "py" | "pyi" | "pyw" => Language::Python,
            "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "cs" => Language::CSharp,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h" => Language::Cpp,
            "c" => Language::C,
            "rb" | "rake" => Language::Ruby,
            "swift" => Language::Swift,
            "kt" | "kts" => Language::Kotlin,
            "php" => Language::Php,
            "sh" | "bash" | "zsh" | "fish" => Language::Shell,
            "sql" => Language::Sql,
            "html" | "htm" => Language::Html,
            "css" | "scss" | "sass" | "less" => Language::Css,
            "md" | "markdown" => Language::Markdown,
            "json" | "jsonc" => Language::Json,
            "yaml" | "yml" => Language::Yaml,
            "toml" => Language::Toml,
            _ => Language::Unknown,
        }
    }

    /// Detect language from file path
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Language::Unknown)
    }

    /// Get the language name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::TypeScript => "TypeScript",
            Language::JavaScript => "JavaScript",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::CSharp => "C#",
            Language::Cpp => "C++",
            Language::C => "C",
            Language::Ruby => "Ruby",
            Language::Swift => "Swift",
            Language::Kotlin => "Kotlin",
            Language::Php => "PHP",
            Language::Shell => "Shell",
            Language::Sql => "SQL",
            Language::Html => "HTML",
            Language::Css => "CSS",
            Language::Markdown => "Markdown",
            Language::Json => "JSON",
            Language::Yaml => "YAML",
            Language::Toml => "TOML",
            Language::Unknown => "Unknown",
        }
    }

    /// Check if this is a systems programming language
    pub fn is_systems_language(&self) -> bool {
        matches!(self, Language::Rust | Language::C | Language::Cpp | Language::Go)
    }

    /// Check if this is a scripting language
    pub fn is_scripting_language(&self) -> bool {
        matches!(
            self,
            Language::Python | Language::JavaScript | Language::Ruby | Language::Php | Language::Shell
        )
    }

    /// Check if this is a markup/config language
    pub fn is_markup_or_config(&self) -> bool {
        matches!(
            self,
            Language::Html
                | Language::Css
                | Language::Markdown
                | Language::Json
                | Language::Yaml
                | Language::Toml
                | Language::Sql
        )
    }
}

/// Language-specific prompt fragments
pub struct LanguagePrompts;

impl LanguagePrompts {
    /// Get the full guidance for a language
    pub fn for_language(lang: Language) -> &'static str {
        match lang {
            Language::Rust => Self::RUST,
            Language::Python => Self::PYTHON,
            Language::TypeScript => Self::TYPESCRIPT,
            Language::JavaScript => Self::JAVASCRIPT,
            Language::Go => Self::GO,
            Language::Java => Self::JAVA,
            Language::CSharp => Self::CSHARP,
            Language::Cpp => Self::CPP,
            Language::C => Self::C,
            Language::Ruby => Self::RUBY,
            Language::Swift => Self::SWIFT,
            Language::Kotlin => Self::KOTLIN,
            Language::Shell => Self::SHELL,
            _ => Self::GENERIC,
        }
    }

    /// Rust-specific guidance
    pub const RUST: &'static str = r#"## Rust Guidelines

- Follow Rust 2021 edition conventions
- Use `Result` and `Option` for error handling, avoid panics
- Prefer `&str` over `String` for function parameters
- Use `impl Trait` for return types when appropriate
- Follow RFC 430 naming: treat acronyms as words (e.g., `LlmClient` not `LLMClient`)
- Use `thiserror` for library errors, `anyhow` for applications
- Prefer iterators over explicit loops
- Use `#[derive]` for common traits
- Keep unsafe code minimal and well-documented
- Use `clippy` suggestions as guidance"#;

    /// Python-specific guidance
    pub const PYTHON: &'static str = r#"## Python Guidelines

- Use Python 3.10+ features when appropriate
- Add type hints for function signatures
- Use `dataclasses` or `pydantic` for data structures
- Prefer `pathlib.Path` over string paths
- Use context managers for resource handling
- Follow PEP 8 style guidelines
- Use `async/await` for I/O-bound operations
- Prefer list comprehensions over map/filter
- Use `logging` module instead of print for debugging
- Handle exceptions specifically, avoid bare `except`"#;

    /// TypeScript-specific guidance
    pub const TYPESCRIPT: &'static str = r#"## TypeScript Guidelines

- Use strict mode (`strict: true` in tsconfig)
- Prefer `interface` over `type` for object shapes
- Use `unknown` instead of `any` when type is uncertain
- Leverage discriminated unions for state management
- Use `const` assertions for literal types
- Prefer `async/await` over raw Promises
- Use optional chaining (`?.`) and nullish coalescing (`??`)
- Export types alongside implementations
- Use `readonly` for immutable properties
- Avoid type assertions (`as`) when possible"#;

    /// JavaScript-specific guidance
    pub const JAVASCRIPT: &'static str = r#"## JavaScript Guidelines

- Use ES2020+ features (optional chaining, nullish coalescing)
- Prefer `const` and `let` over `var`
- Use arrow functions for callbacks
- Prefer `async/await` over callbacks and `.then()`
- Use destructuring for cleaner code
- Use template literals for string interpolation
- Prefer `===` over `==` for comparisons
- Use `Array.prototype` methods (map, filter, reduce)
- Handle errors with try/catch in async code
- Use modules (import/export) over CommonJS when possible"#;

    /// Go-specific guidance
    pub const GO: &'static str = r#"## Go Guidelines

- Follow effective Go conventions
- Return errors as the last return value
- Use `error` interface, wrap with `fmt.Errorf` and `%w`
- Prefer composition over inheritance
- Use interfaces for abstraction, keep them small
- Use goroutines and channels for concurrency
- Always handle errors, don't ignore with `_`
- Use `context.Context` for cancellation and timeouts
- Prefer table-driven tests
- Use `defer` for cleanup operations"#;

    /// Java-specific guidance
    pub const JAVA: &'static str = r#"## Java Guidelines

- Use Java 17+ features when appropriate
- Prefer records for immutable data classes
- Use `Optional` instead of null for optional values
- Leverage streams for collection processing
- Use `var` for local variable type inference
- Follow SOLID principles
- Use dependency injection
- Prefer composition over inheritance
- Use `try-with-resources` for AutoCloseable
- Write unit tests with JUnit 5"#;

    /// C#-specific guidance
    pub const CSHARP: &'static str = r#"## C# Guidelines

- Use C# 10+ features when appropriate
- Use nullable reference types
- Prefer records for immutable data
- Use `async/await` for asynchronous operations
- Leverage LINQ for collection operations
- Use pattern matching for type checks
- Prefer `using` declarations over statements
- Use dependency injection
- Follow .NET naming conventions
- Use `IDisposable` pattern for resource cleanup"#;

    /// C++-specific guidance
    pub const CPP: &'static str = r#"## C++ Guidelines

- Use C++17 or later features
- Follow RAII for resource management
- Use smart pointers (`unique_ptr`, `shared_ptr`)
- Prefer `std::string_view` for read-only strings
- Use `const` and `constexpr` liberally
- Prefer range-based for loops
- Use `std::optional` for optional values
- Avoid raw `new`/`delete`
- Use `[[nodiscard]]` for important return values
- Follow the Rule of Zero/Five"#;

    /// C-specific guidance
    pub const C: &'static str = r#"## C Guidelines

- Use C11 or later features when available
- Always check return values
- Free allocated memory, avoid leaks
- Use `const` for read-only parameters
- Prefer `size_t` for sizes and indices
- Initialize variables at declaration
- Use `static` for file-local functions
- Check array bounds
- Use `enum` for related constants
- Document ownership and lifetime requirements"#;

    /// Ruby-specific guidance
    pub const RUBY: &'static str = r#"## Ruby Guidelines

- Follow Ruby style guide conventions
- Use blocks and iterators idiomatically
- Prefer symbols over strings for identifiers
- Use `attr_reader`/`attr_accessor` appropriately
- Leverage duck typing
- Use modules for mixins
- Prefer `unless` for negative conditions
- Use `&.` safe navigation operator
- Write specs with RSpec
- Use `freeze` for immutable strings"#;

    /// Swift-specific guidance
    pub const SWIFT: &'static str = r#"## Swift Guidelines

- Use Swift 5.5+ features (async/await, actors)
- Prefer `let` over `var` for immutability
- Use optionals properly, avoid force unwrapping
- Leverage value types (structs, enums)
- Use protocol-oriented programming
- Use `guard` for early returns
- Prefer `if let` and `guard let` for optional binding
- Use `Result` type for error handling
- Follow Swift API design guidelines
- Use property wrappers when appropriate"#;

    /// Kotlin-specific guidance
    pub const KOTLIN: &'static str = r#"## Kotlin Guidelines

- Use Kotlin idioms (data classes, sealed classes)
- Prefer `val` over `var` for immutability
- Use null safety features (`?.`, `?:`, `!!`)
- Leverage extension functions
- Use coroutines for async operations
- Prefer `when` over `if-else` chains
- Use scope functions (`let`, `apply`, `also`, `run`)
- Use `sealed class` for restricted hierarchies
- Prefer expression bodies for simple functions
- Use `object` for singletons"#;

    /// Shell-specific guidance
    pub const SHELL: &'static str = r#"## Shell Guidelines

- Use `#!/usr/bin/env bash` for portability
- Quote variables to prevent word splitting
- Use `set -euo pipefail` for safer scripts
- Check command existence before using
- Use `$()` over backticks for command substitution
- Use `[[ ]]` over `[ ]` for conditionals
- Handle signals with `trap`
- Use functions for reusable code
- Validate inputs and arguments
- Use `shellcheck` for linting"#;

    /// Generic guidance for unknown languages
    pub const GENERIC: &'static str = r#"## General Guidelines

- Follow the existing code style in the project
- Write clear, self-documenting code
- Handle errors appropriately
- Keep functions focused and small
- Use meaningful variable and function names
- Add comments only where logic isn't obvious
- Write tests for new functionality
- Consider edge cases and error conditions"#;

    /// Get a compact hint for the language
    pub fn compact_hint(lang: Language) -> &'static str {
        match lang {
            Language::Rust => "Rust: Use Result/Option, follow RFC 430 naming",
            Language::Python => "Python: Add type hints, use async/await for I/O",
            Language::TypeScript => "TypeScript: Use strict mode, prefer interfaces",
            Language::JavaScript => "JavaScript: Use const/let, async/await",
            Language::Go => "Go: Return errors last, use context for cancellation",
            Language::Java => "Java: Use Optional, prefer records for data",
            Language::CSharp => "C#: Use nullable refs, async/await, LINQ",
            Language::Cpp => "C++: Use RAII, smart pointers, const",
            Language::C => "C: Check returns, free memory, use const",
            Language::Ruby => "Ruby: Use blocks, symbols, duck typing",
            Language::Swift => "Swift: Use optionals safely, prefer let",
            Language::Kotlin => "Kotlin: Use null safety, coroutines, sealed classes",
            Language::Shell => "Shell: Quote vars, use set -euo pipefail",
            _ => "Follow existing code style and conventions",
        }
    }
}

/// Detect primary language from a list of file paths
pub fn detect_primary_language(paths: &[&Path]) -> Language {
    use std::collections::HashMap;

    let mut counts: HashMap<Language, usize> = HashMap::new();

    for path in paths {
        let lang = Language::from_path(path);
        if lang != Language::Unknown && !lang.is_markup_or_config() {
            *counts.entry(lang).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(lang, _)| lang)
        .unwrap_or(Language::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

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
