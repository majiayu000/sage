//! Diff utilities for checkpoint system
//!
//! This module provides utilities for detecting changes between checkpoints
//! and generating file snapshots.

use crate::error::{SageError, SageResult};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::types::{FileSnapshot, FileState};

/// File change detector
pub struct ChangeDetector {
    /// Base directory for file operations
    base_dir: PathBuf,
    /// File extensions to track (empty = all)
    tracked_extensions: HashSet<String>,
    /// Directories to exclude
    excluded_dirs: HashSet<PathBuf>,
    /// Maximum file size to capture inline (default: 1MB)
    max_inline_size: u64,
}

impl ChangeDetector {
    /// Create a new change detector
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            tracked_extensions: HashSet::new(),
            excluded_dirs: Self::default_excluded_dirs(),
            max_inline_size: 1024 * 1024, // 1MB
        }
    }

    /// Default excluded directories
    fn default_excluded_dirs() -> HashSet<PathBuf> {
        [
            ".git",
            "node_modules",
            "target",
            ".sage",
            "__pycache__",
            ".venv",
            "venv",
            ".env",
            "dist",
            "build",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }

    /// Track only specific file extensions
    pub fn with_extensions(
        mut self,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.tracked_extensions = extensions.into_iter().map(Into::into).collect();
        self
    }

    /// Add excluded directory
    pub fn exclude_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.excluded_dirs.insert(dir.into());
        self
    }

    /// Set maximum inline file size
    pub fn with_max_inline_size(mut self, size: u64) -> Self {
        self.max_inline_size = size;
        self
    }

    /// Check if a path should be tracked
    fn should_track(&self, path: &Path) -> bool {
        // Check excluded directories
        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_path = PathBuf::from(name);
                if self.excluded_dirs.contains(&name_path) {
                    return false;
                }
            }
        }

        // Check extensions if filter is set
        if !self.tracked_extensions.is_empty() {
            if let Some(ext) = path.extension() {
                return self
                    .tracked_extensions
                    .contains(ext.to_string_lossy().as_ref());
            }
            return false;
        }

        true
    }

    /// Compute SHA-256 hash of content
    pub fn compute_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Capture current state of a single file
    pub async fn capture_file(&self, path: &Path) -> SageResult<Option<FileSnapshot>> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        };

        let relative_path = if path.is_absolute() {
            path.strip_prefix(&self.base_dir)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| path.to_path_buf())
        } else {
            path.to_path_buf()
        };

        if !self.should_track(&relative_path) {
            return Ok(None);
        }

        if !full_path.exists() {
            return Ok(None);
        }

        let metadata = fs::metadata(&full_path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read file metadata: {}", e)))?;

        if metadata.is_dir() {
            return Ok(None);
        }

        let size = metadata.len();
        let _permissions = Self::get_permissions(&metadata);

        // Read content if within size limit
        let (content, content_hash) = if size <= self.max_inline_size {
            match fs::read_to_string(&full_path).await {
                Ok(c) => {
                    let hash = Self::compute_hash(&c);
                    (Some(c), Some(hash))
                }
                Err(_) => (None, None), // Binary file or read error
            }
        } else {
            (None, None)
        };

        Ok(Some(
            FileSnapshot::new(
                relative_path,
                FileState::Exists {
                    content,
                    content_ref: None,
                },
            )
            .with_size(size)
            .with_hash(content_hash.unwrap_or_default()),
        ))
    }

    /// Get file permissions (Unix only)
    #[cfg(unix)]
    fn get_permissions(metadata: &std::fs::Metadata) -> Option<u32> {
        use std::os::unix::fs::PermissionsExt;
        Some(metadata.permissions().mode())
    }

    #[cfg(not(unix))]
    fn get_permissions(_metadata: &std::fs::Metadata) -> Option<u32> {
        None
    }

    /// Capture current state of multiple files
    pub async fn capture_files(&self, paths: &[PathBuf]) -> SageResult<Vec<FileSnapshot>> {
        let mut snapshots = Vec::new();

        for path in paths {
            if let Some(snapshot) = self.capture_file(path).await? {
                snapshots.push(snapshot);
            }
        }

        Ok(snapshots)
    }

    /// Scan directory and capture all tracked files
    pub async fn scan_directory(&self, dir: &Path) -> SageResult<Vec<FileSnapshot>> {
        let full_dir = if dir.is_absolute() {
            dir.to_path_buf()
        } else {
            self.base_dir.join(dir)
        };

        let mut snapshots = Vec::new();
        self.scan_recursive(&full_dir, &mut snapshots).await?;
        Ok(snapshots)
    }

    /// Recursive directory scanning
    async fn scan_recursive(
        &self,
        dir: &Path,
        snapshots: &mut Vec<FileSnapshot>,
    ) -> SageResult<()> {
        let relative_dir = dir.strip_prefix(&self.base_dir).unwrap_or(dir);

        // Check if directory should be excluded
        if let Some(name) = relative_dir.file_name() {
            if self.excluded_dirs.contains(&PathBuf::from(name)) {
                return Ok(());
            }
        }

        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            SageError::storage(format!("Failed to read directory {:?}: {}", dir, e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::storage(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            let metadata = entry
                .metadata()
                .await
                .map_err(|e| SageError::storage(format!("Failed to read metadata: {}", e)))?;

            if metadata.is_dir() {
                Box::pin(self.scan_recursive(&path, snapshots)).await?;
            } else if metadata.is_file() {
                if let Some(snapshot) = self.capture_file(&path).await? {
                    snapshots.push(snapshot);
                }
            }
        }

        Ok(())
    }

    /// Compare two sets of file snapshots and generate changes
    pub fn compare_snapshots(before: &[FileSnapshot], after: &[FileSnapshot]) -> Vec<FileChange> {
        let before_map: HashMap<_, _> = before.iter().map(|f| (&f.path, f)).collect();
        let after_map: HashMap<_, _> = after.iter().map(|f| (&f.path, f)).collect();

        let all_paths: HashSet<_> = before_map.keys().chain(after_map.keys()).collect();
        let mut changes = Vec::new();

        for path in all_paths {
            match (before_map.get(path), after_map.get(path)) {
                (Some(before_file), Some(after_file)) => {
                    // Check if content changed
                    if before_file.content_hash != after_file.content_hash {
                        changes.push(FileChange::Modified {
                            path: (*path).clone(),
                            before: (*before_file).clone(),
                            after: (*after_file).clone(),
                        });
                    }
                }
                (Some(before_file), None) => {
                    changes.push(FileChange::Deleted {
                        path: (*path).clone(),
                        snapshot: (*before_file).clone(),
                    });
                }
                (None, Some(after_file)) => {
                    changes.push(FileChange::Created {
                        path: (*path).clone(),
                        snapshot: (*after_file).clone(),
                    });
                }
                (None, None) => unreachable!(),
            }
        }

        changes
    }

    /// Create file snapshots from changes
    pub fn changes_to_snapshots(changes: &[FileChange]) -> Vec<FileSnapshot> {
        changes
            .iter()
            .map(|change| match change {
                FileChange::Created { snapshot, .. } => FileSnapshot::new(
                    snapshot.path.clone(),
                    FileState::Created {
                        content: Self::extract_content(&snapshot.state),
                        content_ref: None,
                    },
                )
                .with_size(snapshot.size)
                .with_hash(snapshot.content_hash.clone().unwrap_or_default()),

                FileChange::Modified { before, after, .. } => FileSnapshot::new(
                    after.path.clone(),
                    FileState::Modified {
                        original_content: Self::extract_content(&before.state),
                        original_content_ref: None,
                        new_content: Self::extract_content(&after.state),
                        new_content_ref: None,
                    },
                )
                .with_size(after.size)
                .with_hash(after.content_hash.clone().unwrap_or_default()),

                FileChange::Deleted { path, snapshot } => {
                    FileSnapshot::new(path.clone(), FileState::Deleted).with_size(snapshot.size)
                }
            })
            .collect()
    }

    /// Extract content from FileState
    fn extract_content(state: &FileState) -> Option<String> {
        match state {
            FileState::Exists { content, .. } => content.clone(),
            FileState::Created { content, .. } => content.clone(),
            FileState::Modified { new_content, .. } => new_content.clone(),
            FileState::Deleted => None,
        }
    }
}

/// Represents a file change
#[derive(Debug, Clone)]
pub enum FileChange {
    /// File was created
    Created {
        path: PathBuf,
        snapshot: FileSnapshot,
    },
    /// File was modified
    Modified {
        path: PathBuf,
        before: FileSnapshot,
        after: FileSnapshot,
    },
    /// File was deleted
    Deleted {
        path: PathBuf,
        snapshot: FileSnapshot,
    },
}

impl FileChange {
    /// Get the path of the changed file
    pub fn path(&self) -> &Path {
        match self {
            Self::Created { path, .. } => path,
            Self::Modified { path, .. } => path,
            Self::Deleted { path, .. } => path,
        }
    }

    /// Check if this is a creation
    pub fn is_created(&self) -> bool {
        matches!(self, Self::Created { .. })
    }

    /// Check if this is a modification
    pub fn is_modified(&self) -> bool {
        matches!(self, Self::Modified { .. })
    }

    /// Check if this is a deletion
    pub fn is_deleted(&self) -> bool {
        matches!(self, Self::Deleted { .. })
    }
}

/// Simple text diff implementation
#[derive(Debug, Clone)]
pub struct TextDiff {
    pub hunks: Vec<DiffHunk>,
}

/// A diff hunk
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

/// A diff line
#[derive(Debug, Clone)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

impl TextDiff {
    /// Compute diff between two strings
    pub fn compute(old: &str, new: &str) -> Self {
        let old_lines: Vec<_> = old.lines().collect();
        let new_lines: Vec<_> = new.lines().collect();

        // Simple LCS-based diff
        let hunks = Self::compute_hunks(&old_lines, &new_lines);
        Self { hunks }
    }

    /// Compute hunks using simple LCS algorithm
    fn compute_hunks(old: &[&str], new: &[&str]) -> Vec<DiffHunk> {
        // For simplicity, use a basic approach
        // In production, would use proper LCS/Myers diff
        let mut hunks = Vec::new();
        let mut lines = Vec::new();

        let mut old_idx = 0;
        let mut new_idx = 0;

        while old_idx < old.len() || new_idx < new.len() {
            if old_idx < old.len() && new_idx < new.len() {
                if old[old_idx] == new[new_idx] {
                    lines.push(DiffLine::Context(old[old_idx].to_string()));
                    old_idx += 1;
                    new_idx += 1;
                } else {
                    // Simple: mark old as removed, new as added
                    lines.push(DiffLine::Removed(old[old_idx].to_string()));
                    old_idx += 1;
                    if new_idx < new.len() {
                        lines.push(DiffLine::Added(new[new_idx].to_string()));
                        new_idx += 1;
                    }
                }
            } else if old_idx < old.len() {
                lines.push(DiffLine::Removed(old[old_idx].to_string()));
                old_idx += 1;
            } else {
                lines.push(DiffLine::Added(new[new_idx].to_string()));
                new_idx += 1;
            }
        }

        if !lines.is_empty() {
            hunks.push(DiffHunk {
                old_start: 1,
                old_count: old.len(),
                new_start: 1,
                new_count: new.len(),
                lines,
            });
        }

        hunks
    }

    /// Format diff as unified diff string
    pub fn format_unified(&self) -> String {
        let mut output = String::new();

        for hunk in &self.hunks {
            output.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
            ));

            for line in &hunk.lines {
                match line {
                    DiffLine::Context(s) => output.push_str(&format!(" {}\n", s)),
                    DiffLine::Added(s) => output.push_str(&format!("+{}\n", s)),
                    DiffLine::Removed(s) => output.push_str(&format!("-{}\n", s)),
                }
            }
        }

        output
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.hunks.iter().any(|h| {
            h.lines
                .iter()
                .any(|l| matches!(l, DiffLine::Added(_) | DiffLine::Removed(_)))
        })
    }

    /// Count added lines
    pub fn added_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| matches!(l, DiffLine::Added(_)))
            .count()
    }

    /// Count removed lines
    pub fn removed_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| matches!(l, DiffLine::Removed(_)))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_capture_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, World!").await.unwrap();

        let detector = ChangeDetector::new(temp_dir.path());
        let snapshot = detector.capture_file(&file_path).await.unwrap();

        assert!(snapshot.is_some());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.path, PathBuf::from("test.txt"));
        assert_eq!(snapshot.size, 13);
    }

    #[tokio::test]
    async fn test_capture_file_excluded_dir() {
        let temp_dir = TempDir::new().unwrap();
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir_all(&node_modules).await.unwrap();

        let file_path = node_modules.join("test.js");
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"module.exports = {}").await.unwrap();

        let detector = ChangeDetector::new(temp_dir.path());
        let snapshot = detector.capture_file(&file_path).await.unwrap();

        assert!(snapshot.is_none()); // Should be excluded
    }

    #[tokio::test]
    async fn test_capture_file_extension_filter() {
        let temp_dir = TempDir::new().unwrap();

        let rs_file = temp_dir.path().join("main.rs");
        let txt_file = temp_dir.path().join("notes.txt");

        let mut f1 = File::create(&rs_file).await.unwrap();
        f1.write_all(b"fn main() {}").await.unwrap();

        let mut f2 = File::create(&txt_file).await.unwrap();
        f2.write_all(b"Notes").await.unwrap();

        let detector = ChangeDetector::new(temp_dir.path()).with_extensions(["rs"]);

        let rs_snapshot = detector.capture_file(&rs_file).await.unwrap();
        let txt_snapshot = detector.capture_file(&txt_file).await.unwrap();

        assert!(rs_snapshot.is_some());
        assert!(txt_snapshot.is_none());
    }

    #[tokio::test]
    async fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).await.unwrap();

        let mut f1 = File::create(src_dir.join("main.rs")).await.unwrap();
        f1.write_all(b"fn main() {}").await.unwrap();

        let mut f2 = File::create(src_dir.join("lib.rs")).await.unwrap();
        f2.write_all(b"pub mod test;").await.unwrap();

        let detector = ChangeDetector::new(temp_dir.path());
        let snapshots = detector.scan_directory(temp_dir.path()).await.unwrap();

        assert_eq!(snapshots.len(), 2);
    }

    #[test]
    fn test_compare_snapshots_created() {
        let before: Vec<FileSnapshot> = vec![];
        let after = vec![FileSnapshot::new(
            "new.txt",
            FileState::Exists {
                content: Some("New content".to_string()),
                content_ref: None,
            },
        )];

        let changes = ChangeDetector::compare_snapshots(&before, &after);

        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_created());
    }

    #[test]
    fn test_compare_snapshots_deleted() {
        let before = vec![FileSnapshot::new(
            "old.txt",
            FileState::Exists {
                content: Some("Old content".to_string()),
                content_ref: None,
            },
        )];
        let after: Vec<FileSnapshot> = vec![];

        let changes = ChangeDetector::compare_snapshots(&before, &after);

        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_deleted());
    }

    #[test]
    fn test_compare_snapshots_modified() {
        let before = vec![
            FileSnapshot::new(
                "file.txt",
                FileState::Exists {
                    content: Some("Before".to_string()),
                    content_ref: None,
                },
            )
            .with_hash("hash1"),
        ];

        let after = vec![
            FileSnapshot::new(
                "file.txt",
                FileState::Exists {
                    content: Some("After".to_string()),
                    content_ref: None,
                },
            )
            .with_hash("hash2"),
        ];

        let changes = ChangeDetector::compare_snapshots(&before, &after);

        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_modified());
    }

    #[test]
    fn test_text_diff() {
        let old = "line1\nline2\nline3";
        let new = "line1\nmodified\nline3";

        let diff = TextDiff::compute(old, new);

        assert!(diff.has_changes());
        assert!(diff.added_count() > 0);
        assert!(diff.removed_count() > 0);
    }

    #[test]
    fn test_text_diff_no_changes() {
        let text = "line1\nline2\nline3";
        let diff = TextDiff::compute(text, text);

        assert!(!diff.has_changes());
    }

    #[test]
    fn test_text_diff_format_unified() {
        let old = "a\nb\nc";
        let new = "a\nx\nc";

        let diff = TextDiff::compute(old, new);
        let formatted = diff.format_unified();

        assert!(formatted.contains("@@"));
        assert!(formatted.contains("-b"));
        assert!(formatted.contains("+x"));
    }

    #[test]
    fn test_changes_to_snapshots() {
        let created = FileChange::Created {
            path: PathBuf::from("new.txt"),
            snapshot: FileSnapshot::new(
                "new.txt",
                FileState::Exists {
                    content: Some("content".to_string()),
                    content_ref: None,
                },
            ),
        };

        let snapshots = ChangeDetector::changes_to_snapshots(&[created]);

        assert_eq!(snapshots.len(), 1);
        if let FileState::Created { content, .. } = &snapshots[0].state {
            assert_eq!(content, &Some("content".to_string()));
        } else {
            panic!("Expected Created state");
        }
    }

    #[test]
    fn test_file_change_methods() {
        let change = FileChange::Created {
            path: PathBuf::from("test.txt"),
            snapshot: FileSnapshot::new("test.txt", FileState::Deleted),
        };

        assert_eq!(change.path(), Path::new("test.txt"));
        assert!(change.is_created());
        assert!(!change.is_modified());
        assert!(!change.is_deleted());
    }
}
