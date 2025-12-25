//! File discovery and indexing logic for codebase retrieval

use sage_core::tools::base::ToolError;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::CodebaseRetrievalTool;
use super::types::SearchAnalysis;

impl CodebaseRetrievalTool {
    /// Find all relevant files in the directory
    pub(super) fn find_relevant_files(
        &self,
        dir: &Path,
        search_analysis: &SearchAnalysis,
    ) -> Result<Vec<PathBuf>, ToolError> {
        let mut files = Vec::new();
        self.collect_files_recursive(dir, &mut files, 0, 5)?; // Max depth 5

        // Filter by file patterns if specified
        if !search_analysis.file_patterns.is_empty() {
            files.retain(|path| {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                search_analysis
                    .file_patterns
                    .iter()
                    .any(|pattern| file_name.contains(pattern))
            });
        }

        Ok(files)
    }

    pub(super) fn collect_files_recursive(
        &self,
        dir: &Path,
        files: &mut Vec<PathBuf>,
        depth: usize,
        max_depth: usize,
    ) -> Result<(), ToolError> {
        if depth > max_depth {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read directory '{}': {}",
                dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                ToolError::ExecutionFailed(format!(
                    "Failed to read directory entry in '{}': {}",
                    dir.display(),
                    e
                ))
            })?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                    if self
                        .supported_extensions
                        .contains(&extension.to_lowercase())
                    {
                        // Check file size
                        if let Ok(metadata) = fs::metadata(&path) {
                            if metadata.len() <= self.max_file_size as u64 {
                                files.push(path);
                            }
                        }
                    }
                }
            } else if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip common directories that usually don't contain source code
                if !self.should_skip_directory(dir_name) {
                    self.collect_files_recursive(&path, files, depth + 1, max_depth)?;
                }
            }
        }

        Ok(())
    }

    pub(super) fn should_skip_directory(&self, dir_name: &str) -> bool {
        matches!(
            dir_name,
            ".git"
                | ".svn"
                | ".hg"
                | "node_modules"
                | "target"
                | "build"
                | "dist"
                | "__pycache__"
                | ".pytest_cache"
                | ".idea"
                | ".vscode"
                | "coverage"
                | "htmlcov"
                | "tmp"
                | "temp"
                | "cache"
        ) || dir_name.starts_with('.')
    }
}

pub fn get_supported_extensions() -> HashSet<String> {
    let mut supported_extensions = HashSet::new();
    // Programming languages
    supported_extensions.insert("rs".to_string());
    supported_extensions.insert("py".to_string());
    supported_extensions.insert("js".to_string());
    supported_extensions.insert("ts".to_string());
    supported_extensions.insert("java".to_string());
    supported_extensions.insert("cpp".to_string());
    supported_extensions.insert("c".to_string());
    supported_extensions.insert("h".to_string());
    supported_extensions.insert("go".to_string());
    supported_extensions.insert("rb".to_string());
    supported_extensions.insert("php".to_string());
    supported_extensions.insert("cs".to_string());
    supported_extensions.insert("swift".to_string());
    supported_extensions.insert("kt".to_string());
    supported_extensions.insert("scala".to_string());
    supported_extensions.insert("dart".to_string());

    // Config and markup
    supported_extensions.insert("json".to_string());
    supported_extensions.insert("toml".to_string());
    supported_extensions.insert("yaml".to_string());
    supported_extensions.insert("yml".to_string());
    supported_extensions.insert("xml".to_string());
    supported_extensions.insert("md".to_string());
    supported_extensions.insert("txt".to_string());

    supported_extensions
}
