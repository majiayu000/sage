//! Pattern matcher implementation

use std::collections::HashMap;
use std::path::Path;

use super::language_patterns::{
    go_patterns, java_patterns, node_patterns, python_patterns, rust_patterns, universal_patterns,
};
use super::types::{ImportantFile, ImportantFileType, ProjectPattern};
use crate::workspace::detector::LanguageType;

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
        for pattern in universal_patterns() {
            matcher.add_pattern(pattern);
        }

        // Add language-specific patterns
        let lang_patterns = match lang {
            LanguageType::Rust => rust_patterns(),
            LanguageType::TypeScript | LanguageType::JavaScript => node_patterns(),
            LanguageType::Python => python_patterns(),
            LanguageType::Go => go_patterns(),
            LanguageType::Java => java_patterns(),
            _ => Vec::new(),
        };

        for pattern in lang_patterns {
            matcher.add_pattern(pattern);
        }

        matcher
    }

    /// Add a pattern
    pub fn add_pattern(&mut self, pattern: ProjectPattern) {
        self.patterns.push(pattern);
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
                        let exists = files.iter().any(|f| f.path == *glob_pattern);
                        if !exists {
                            files.push(ImportantFile {
                                path: std::path::PathBuf::from(glob_pattern),
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
