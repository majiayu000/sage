//! File statistics collection

use std::path::Path;

use glob::Pattern;

use super::models::{FileStats, WorkspaceConfig, WorkspaceError};

/// Collect file statistics for a workspace
pub fn collect_stats(root: &Path, config: &WorkspaceConfig) -> Result<FileStats, WorkspaceError> {
    let mut stats = FileStats::default();
    let mut files_scanned = 0;
    let exclude_patterns = build_exclude_patterns(config);

    scan_directory(
        root,
        root,
        0,
        &mut stats,
        &mut files_scanned,
        config,
        &exclude_patterns,
    )?;

    // Sort largest files
    stats.largest_files.sort_by(|a, b| b.1.cmp(&a.1));
    stats.largest_files.truncate(10);

    Ok(stats)
}

/// Recursively scan a directory and collect statistics
pub fn scan_directory(
    root: &Path,
    dir: &Path,
    depth: usize,
    stats: &mut FileStats,
    files_scanned: &mut usize,
    config: &WorkspaceConfig,
    exclude_patterns: &[ExcludePattern],
) -> Result<(), WorkspaceError> {
    if depth > config.max_depth || *files_scanned >= config.max_files {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden files if configured
        if !config.include_hidden && file_name.starts_with('.') {
            continue;
        }

        // Skip excluded patterns
        if should_exclude(file_name, exclude_patterns) {
            continue;
        }

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            stats.total_directories += 1;
            scan_directory(
                root,
                &path,
                depth + 1,
                stats,
                files_scanned,
                config,
                exclude_patterns,
            )?;
        } else if file_type.is_file() {
            stats.total_files += 1;
            *files_scanned += 1;

            // Count by extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                *stats.by_extension.entry(ext.to_lowercase()).or_default() += 1;

                // Map extension to language
                if let Some(lang) = extension_to_language(ext) {
                    *stats.by_language.entry(lang.to_string()).or_default() += 1;
                }
            }

            // Track file size
            if let Ok(metadata) = path.metadata() {
                let size = metadata.len();
                if stats.largest_files.len() < 10
                    || size > stats.largest_files.last().map(|f| f.1).unwrap_or(0)
                {
                    let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                    stats.largest_files.push((relative, size));
                }

                // Estimate lines (rough: 40 bytes per line average)
                stats.total_lines += (size / 40) as usize;
            }
        }
    }

    Ok(())
}

/// Check if a file/directory should be excluded
fn should_exclude(name: &str, exclude_patterns: &[ExcludePattern]) -> bool {
    exclude_patterns.iter().any(|pattern| match pattern {
        ExcludePattern::Exact(exact) => name == exact,
        ExcludePattern::Glob(glob) => glob.matches(name),
    })
}

fn build_exclude_patterns(config: &WorkspaceConfig) -> Vec<ExcludePattern> {
    config
        .exclude_patterns
        .iter()
        .filter_map(|pattern| {
            if pattern.contains('*') {
                Pattern::new(pattern)
                    .ok()
                    .map(ExcludePattern::Glob)
            } else {
                Some(ExcludePattern::Exact(pattern.clone()))
            }
        })
        .collect()
}

/// Pattern for excluding files from workspace scanning
pub enum ExcludePattern {
    /// Exact string match
    Exact(String),
    /// Glob pattern match
    Glob(Pattern),
}

/// Map file extension to programming language
pub fn extension_to_language(ext: &str) -> Option<&str> {
    match ext.to_lowercase().as_str() {
        "rs" => Some("Rust"),
        "ts" | "tsx" => Some("TypeScript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript"),
        "py" | "pyi" => Some("Python"),
        "go" => Some("Go"),
        "java" => Some("Java"),
        "kt" | "kts" => Some("Kotlin"),
        "scala" | "sc" => Some("Scala"),
        "cs" => Some("C#"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("C++"),
        "c" | "h" => Some("C"),
        "rb" => Some("Ruby"),
        "php" => Some("PHP"),
        "swift" => Some("Swift"),
        "sh" | "bash" | "zsh" => Some("Shell"),
        "sql" => Some("SQL"),
        "html" | "htm" => Some("HTML"),
        "css" | "scss" | "sass" | "less" => Some("CSS"),
        "json" => Some("JSON"),
        "yaml" | "yml" => Some("YAML"),
        "toml" => Some("TOML"),
        "md" | "markdown" => Some("Markdown"),
        _ => None,
    }
}
