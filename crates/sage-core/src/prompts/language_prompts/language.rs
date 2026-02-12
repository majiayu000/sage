//! Programming language detection and classification

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
        matches!(
            self,
            Language::Rust | Language::C | Language::Cpp | Language::Go
        )
    }

    /// Check if this is a scripting language
    pub fn is_scripting_language(&self) -> bool {
        matches!(
            self,
            Language::Python
                | Language::JavaScript
                | Language::Ruby
                | Language::Php
                | Language::Shell
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
