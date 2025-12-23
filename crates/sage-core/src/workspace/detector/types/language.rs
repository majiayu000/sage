//! Programming language type definitions

use serde::{Deserialize, Serialize};

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
