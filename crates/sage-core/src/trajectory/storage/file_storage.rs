//! File-based trajectory storage implementation

use crate::error::{SageError, SageResult};
use crate::types::Id;
use std::path::{Path, PathBuf};

use super::types::RotationConfig;

/// File-based trajectory storage.
///
/// Stores trajectory records as JSON files on disk, with optional gzip compression
/// and automatic file rotation to manage disk space.
///
/// # Features
///
/// - **Compression**: Optional gzip compression reduces file sizes by 5-10x
/// - **Rotation**: Automatic deletion of old files based on count or size limits
/// - **Flexible paths**: Supports both directory-based (multiple files) and single-file modes
/// - **Transparent loading**: Automatically detects and handles both compressed and uncompressed files
///
/// # Examples
///
/// ```no_run
/// use sage_core::trajectory::storage::{FileStorage, RotationConfig};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Basic usage - directory of JSON files
/// let storage = FileStorage::new("trajectories")?;
///
/// // With compression enabled
/// let storage = FileStorage::with_compression("trajectories", true)?;
///
/// // With rotation - keep last 10 files
/// let rotation = RotationConfig::with_max_trajectories(10);
/// let storage = FileStorage::with_config("trajectories", true, rotation)?;
/// # Ok(())
/// # }
/// ```
pub struct FileStorage {
    base_path: PathBuf,
    enable_compression: bool,
    rotation_config: RotationConfig,
}

impl FileStorage {
    /// Create a new file storage without compression or rotation.
    pub fn new<P: AsRef<Path>>(path: P) -> SageResult<Self> {
        Self::with_config(path, false, RotationConfig::default())
    }

    /// Create a new file storage with optional compression
    pub fn with_compression<P: AsRef<Path>>(path: P, enable_compression: bool) -> SageResult<Self> {
        Self::with_config(path, enable_compression, RotationConfig::default())
    }

    /// Create a new file storage with compression and rotation configuration
    pub fn with_config<P: AsRef<Path>>(
        path: P,
        enable_compression: bool,
        rotation_config: RotationConfig,
    ) -> SageResult<Self> {
        let base_path = path.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SageError::config(format!("Failed to create trajectory directory: {}", e))
            })?;
        }

        Ok(Self {
            base_path,
            enable_compression,
            rotation_config,
        })
    }

    /// Get the base path for this storage.
    ///
    /// Returns the directory or file path configured for this storage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::trajectory::storage::FileStorage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = FileStorage::new("trajectories")?;
    /// println!("Storage path: {}", storage.path().display());
    /// # Ok(())
    /// # }
    /// ```
    pub fn path(&self) -> &Path {
        &self.base_path
    }

    /// Get the file path for a trajectory ID.
    ///
    /// In directory mode, generates a path with the trajectory ID.
    /// In single-file mode, returns the base path.
    pub(super) fn get_file_path(&self, id: Id) -> PathBuf {
        if self.is_directory_path() {
            self.base_path.join(format!("{}.json", id))
        } else {
            // If base_path is a file, use it directly for single trajectory
            self.base_path.clone()
        }
    }

    /// Determine if base_path should be treated as a directory.
    ///
    /// A path is considered a directory if:
    /// 1. It exists and is a directory, OR
    /// 2. It doesn't exist but has no file extension (assumed to be a directory)
    ///
    /// This allows the storage to work in two modes:
    /// - Directory mode: Multiple trajectory files with timestamp-based names
    /// - File mode: Single trajectory file that gets overwritten
    pub(super) fn is_directory_path(&self) -> bool {
        if self.base_path.exists() {
            self.base_path.is_dir()
        } else {
            // If path doesn't exist, check if it has an extension
            // Paths without extensions are assumed to be directories
            self.base_path.extension().is_none()
        }
    }

    /// Get a reference to the base path
    pub(super) fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get a reference to the rotation config
    pub(super) fn rotation_config(&self) -> &RotationConfig {
        &self.rotation_config
    }

    /// Check if compression is enabled
    pub(super) fn enable_compression(&self) -> bool {
        self.enable_compression
    }
}
