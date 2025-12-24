//! File filtering logic for grep searches

use std::path::Path;

/// Check if a file should be included based on glob pattern
pub fn matches_glob(path: &Path, glob_pattern: &str) -> bool {
    if let Ok(pattern) = glob::Pattern::new(glob_pattern) {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            return pattern.matches(file_name);
        }
    }
    false
}

/// Get file extension for type filtering
pub fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

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

/// Check if a file should be skipped
pub fn should_skip_file(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        // Skip common binary and cache directories
        if path.ancestors().any(|p| {
            if let Some(dir_name) = p.file_name().and_then(|n| n.to_str()) {
                matches!(
                    dir_name,
                    "node_modules"
                        | "target"
                        | ".git"
                        | ".svn"
                        | ".hg"
                        | "dist"
                        | "build"
                        | "__pycache__"
                        | ".pytest_cache"
                        | ".tox"
                        | "venv"
                        | ".venv"
                )
            } else {
                false
            }
        }) {
            return true;
        }

        // Skip common binary extensions
        if let Some(ext) = get_extension(path) {
            if matches!(
                ext.as_str(),
                // Executables and libraries
                "exe" | "dll" | "so" | "dylib" | "a" | "o" | "obj" | "bin"
                    // Data files
                    | "dat" | "db" | "sqlite" | "sqlite3"
                    // Images
                    | "png" | "jpg" | "jpeg" | "gif" | "ico" | "svg" | "bmp" | "tiff" | "webp"
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
            ) {
                return true;
            }
        }

        // Skip hidden files starting with .
        if name.starts_with('.') && name.len() > 1 {
            return true;
        }
    }

    false
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
    fn test_get_extension() {
        assert_eq!(get_extension(Path::new("test.rs")), Some("rs".to_string()));
        assert_eq!(get_extension(Path::new("test.RS")), Some("rs".to_string()));
        assert_eq!(get_extension(Path::new("test")), None);
    }

    #[test]
    fn test_matches_glob() {
        let path = Path::new("test.rs");
        assert!(matches_glob(path, "*.rs"));
        assert!(!matches_glob(path, "*.txt"));
        assert!(matches_glob(path, "test.*"));
    }

    #[test]
    fn test_should_skip_file() {
        assert!(should_skip_file(Path::new(".git/config")));
        assert!(should_skip_file(Path::new("node_modules/package/index.js")));
        assert!(should_skip_file(Path::new("target/debug/app")));
        assert!(should_skip_file(Path::new(".hidden")));
        assert!(should_skip_file(Path::new("image.png")));
        assert!(!should_skip_file(Path::new("test.rs")));
    }
}
