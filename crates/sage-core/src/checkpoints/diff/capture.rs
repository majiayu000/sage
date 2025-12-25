//! File snapshot capture logic

use crate::error::{SageError, SageResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::super::types::{FileSnapshot, FileState};

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
    /// Get the base directory
    pub(super) fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Check if a directory is in the excluded list
    pub(super) fn is_excluded(&self, dir: &Path) -> bool {
        self.excluded_dirs.contains(dir)
    }
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
}
