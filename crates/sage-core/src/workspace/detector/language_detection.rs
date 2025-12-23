//! Language detection logic
//!
//! Detects the primary language of a project by counting source files
//! with different extensions.

use super::types::LanguageType;
use std::collections::HashMap;
use std::path::Path;

/// Detects the primary language by counting files with different extensions
pub(super) fn detect_by_file_count(root: &Path, max_depth: usize) -> LanguageType {
    let mut counts: HashMap<LanguageType, usize> = HashMap::new();

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
        let count = count_files_with_extensions(root, lang.extensions(), max_depth);
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

/// Recursively counts files with given extensions
pub(super) fn count_files_with_extensions(
    root: &Path,
    extensions: &[&str],
    max_depth: usize,
) -> usize {
    let mut count = 0;

    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_string_lossy().as_ref()) {
                    count += 1;
                }
            }
        } else if path.is_dir() && max_depth > 0 {
            // Simple recursive count (limited depth)
            count += count_files_with_extensions(&path, extensions, max_depth - 1);
        }
    }

    count
}
