//! File filtering logic for grep searches
//!
//! Note: Most filtering is now handled by the `ignore` crate which respects
//! .gitignore files. This module provides additional type-based filtering
//! and binary extension detection.

use std::path::Path;

/// Check if a file matches the type filter
pub fn matches_type(path: &Path, type_filter: &str) -> bool {
    if let Some(ext) = get_extension(path) {
        match type_filter {
            "rs" | "rust" => ext == "rs",
            "js" | "javascript" => matches!(ext.as_str(), "js" | "jsx" | "mjs" | "cjs"),
            "ts" | "typescript" => matches!(ext.as_str(), "ts" | "tsx"),
            "py" | "python" => ext == "py",
            "go" => ext == "go",
            "java" => ext == "java",
            "c" => ext == "c",
            "cpp" | "c++" => matches!(ext.as_str(), "cpp" | "cc" | "cxx" | "hpp" | "h"),
            "rb" | "ruby" => ext == "rb",
            "php" => ext == "php",
            "html" => matches!(ext.as_str(), "html" | "htm"),
            "css" => ext == "css",
            "json" => ext == "json",
            "yaml" | "yml" => matches!(ext.as_str(), "yaml" | "yml"),
            "xml" => ext == "xml",
            "md" | "markdown" => matches!(ext.as_str(), "md" | "markdown"),
            "txt" | "text" => ext == "txt",
            "toml" => ext == "toml",
            "sql" => ext == "sql",
            "sh" | "shell" | "bash" => matches!(ext.as_str(), "sh" | "bash" | "zsh"),
            _ => false,
        }
    } else {
        false
    }
}

/// Get file extension for type filtering
fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// Check if a file has a known binary extension
///
/// This is used as an additional filter on top of grep-searcher's
/// BinaryDetection which uses NUL byte heuristics.
pub fn is_binary_extension(path: &Path) -> bool {
    if let Some(ext) = get_extension(path) {
        matches!(
            ext.as_str(),
            // Executables and libraries
            "exe" | "dll" | "so" | "dylib" | "a" | "o" | "obj" | "bin"
                // Data files
                | "dat" | "db" | "sqlite" | "sqlite3"
                // Images
                | "png" | "jpg" | "jpeg" | "gif" | "ico" | "bmp" | "tiff" | "webp"
                // Documents
                | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
                // Archives
                | "zip" | "tar" | "gz" | "bz2" | "xz" | "rar" | "7z" | "tgz"
                // Media
                | "mp3" | "mp4" | "avi" | "mov" | "mkv" | "wav" | "flac" | "ogg" | "webm"
                // Fonts
                | "woff" | "woff2" | "ttf" | "eot" | "otf"
                // Python compiled
                | "pyc" | "pyo" | "pyd"
                // Java compiled
                | "class" | "jar" | "war" | "ear"
                // .NET compiled
                | "pdb"
                // Gettext compiled
                | "mo"
                // Node/npm
                | "node"
                // Misc binary
                | "wasm"
        )
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_matches_type() {
        let path = Path::new("test.rs");
        assert!(matches_type(path, "rust"));
        assert!(matches_type(path, "rs"));
        assert!(!matches_type(path, "python"));

        let path = Path::new("test.tsx");
        assert!(matches_type(path, "typescript"));
        assert!(matches_type(path, "ts"));
    }

    #[test]
    fn test_is_binary_extension() {
        assert!(is_binary_extension(Path::new("image.png")));
        assert!(is_binary_extension(Path::new("app.exe")));
        assert!(is_binary_extension(Path::new("cache.pyc")));
        assert!(is_binary_extension(Path::new("Module.class")));
        assert!(!is_binary_extension(Path::new("code.rs")));
        assert!(!is_binary_extension(Path::new("script.py")));
    }
}
