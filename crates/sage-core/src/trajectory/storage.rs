//! Trajectory storage implementations

use crate::error::{ResultExt, SageError, SageResult};
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use async_trait::async_trait;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::any::Any;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::instrument;

/// Trait for trajectory storage backends.
///
/// Defines the interface for storing, retrieving, and managing trajectory records.
/// Implementations include file-based storage and in-memory storage.
///
/// # Examples
///
/// ```no_run
/// use sage_core::trajectory::storage::{TrajectoryStorage, FileStorage};
/// use sage_core::trajectory::recorder::TrajectoryRecord;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = FileStorage::new("trajectories")?;
///
/// // Save a trajectory
/// # let record = TrajectoryRecord {
/// #     id: uuid::Uuid::new_v4(),
/// #     task: "example".to_string(),
/// #     start_time: "2024-01-01T00:00:00Z".to_string(),
/// #     end_time: "2024-01-01T00:05:00Z".to_string(),
/// #     provider: "test".to_string(),
/// #     model: "test".to_string(),
/// #     max_steps: Some(10),
/// #     llm_interactions: vec![],
/// #     agent_steps: vec![],
/// #     success: true,
/// #     final_result: Some("done".to_string()),
/// #     execution_time: 5.0,
/// # };
/// storage.save(&record).await?;
///
/// // Load it back
/// let loaded = storage.load(record.id).await?;
/// assert!(loaded.is_some());
///
/// // List all trajectories
/// let ids = storage.list().await?;
/// println!("Found {} trajectories", ids.len());
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait TrajectoryStorage: Send + Sync {
    /// Save a trajectory record to storage.
    ///
    /// # Arguments
    ///
    /// * `record` - The trajectory record to save
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage backend is unavailable
    /// - Serialization fails
    /// - File system errors occur (for file-based storage)
    /// - Disk is full
    async fn save(&self, record: &TrajectoryRecord) -> SageResult<()>;

    /// Load a trajectory record by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the trajectory to load
    ///
    /// # Returns
    ///
    /// Returns `Some(record)` if found, `None` if not found.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage backend is unavailable
    /// - Deserialization fails
    /// - File system errors occur
    async fn load(&self, id: Id) -> SageResult<Option<TrajectoryRecord>>;

    /// List all trajectory IDs in storage.
    ///
    /// # Returns
    ///
    /// A vector of all trajectory UUIDs in the storage.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage backend is unavailable
    /// - Directory cannot be read (for file-based storage)
    async fn list(&self) -> SageResult<Vec<Id>>;

    /// Delete a trajectory record by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the trajectory to delete
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Trajectory not found
    /// - Storage backend is unavailable
    /// - File system errors occur
    async fn delete(&self, id: Id) -> SageResult<()>;

    /// Get storage statistics.
    ///
    /// Returns metadata about the storage including total records,
    /// total size, and average record size.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage backend is unavailable
    /// - File system errors occur
    async fn statistics(&self) -> SageResult<StorageStatistics>;

    /// For downcasting to concrete types.
    ///
    /// This method enables dynamic type checking and conversion.
    fn as_any(&self) -> &dyn Any;
}

/// Statistics about trajectory storage.
///
/// Provides metrics about stored trajectories including count and size information.
///
/// # Examples
///
/// ```no_run
/// use sage_core::trajectory::storage::{TrajectoryStorage, FileStorage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = FileStorage::new("trajectories")?;
/// let stats = storage.statistics().await?;
///
/// println!("Total trajectories: {}", stats.total_records);
/// println!("Total size: {} bytes", stats.total_size_bytes);
/// println!("Average size: {} bytes", stats.average_record_size);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct StorageStatistics {
    /// Total number of stored trajectories.
    pub total_records: usize,
    /// Total storage size in bytes.
    pub total_size_bytes: u64,
    /// Average record size in bytes.
    pub average_record_size: u64,
}

/// Rotation configuration for trajectory files.
///
/// Controls automatic deletion of old trajectory files to prevent
/// unbounded storage growth. You can limit by count, total size, or both.
///
/// # Examples
///
/// ```no_run
/// use sage_core::trajectory::storage::{RotationConfig, FileStorage};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Keep only the 10 most recent trajectories
/// let config = RotationConfig::with_max_trajectories(10);
/// let storage = FileStorage::with_config("trajectories", false, config)?;
///
/// // Limit total storage to 100MB
/// let config = RotationConfig::with_total_size_limit(100 * 1024 * 1024);
/// let storage = FileStorage::with_config("trajectories", false, config)?;
///
/// // Apply both limits
/// let config = RotationConfig::with_limits(50, 500 * 1024 * 1024);
/// let storage = FileStorage::with_config("trajectories", false, config)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct RotationConfig {
    /// Maximum number of trajectory files to keep.
    ///
    /// When exceeded, oldest files are deleted. `None` means unlimited.
    pub max_trajectories: Option<usize>,

    /// Maximum total size in bytes for all trajectories.
    ///
    /// When exceeded, oldest files are deleted until under limit. `None` means unlimited.
    pub total_size_limit: Option<u64>,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            max_trajectories: None,
            total_size_limit: None,
        }
    }
}

impl RotationConfig {
    /// Create a rotation config with max trajectories limit.
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum number of trajectory files to keep
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::RotationConfig;
    ///
    /// let config = RotationConfig::with_max_trajectories(20);
    /// assert_eq!(config.max_trajectories, Some(20));
    /// assert_eq!(config.total_size_limit, None);
    /// ```
    pub fn with_max_trajectories(max: usize) -> Self {
        Self {
            max_trajectories: Some(max),
            total_size_limit: None,
        }
    }

    /// Create a rotation config with total size limit.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum total size in bytes for all trajectories
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::RotationConfig;
    ///
    /// let config = RotationConfig::with_total_size_limit(1024 * 1024 * 100); // 100MB
    /// assert_eq!(config.max_trajectories, None);
    /// assert_eq!(config.total_size_limit, Some(1024 * 1024 * 100));
    /// ```
    pub fn with_total_size_limit(limit: u64) -> Self {
        Self {
            max_trajectories: None,
            total_size_limit: Some(limit),
        }
    }

    /// Create a rotation config with both count and size limits.
    ///
    /// Both limits are enforced - whichever is exceeded first triggers rotation.
    ///
    /// # Arguments
    ///
    /// * `max_trajectories` - Maximum number of trajectory files
    /// * `total_size_limit` - Maximum total size in bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::RotationConfig;
    ///
    /// let config = RotationConfig::with_limits(50, 200 * 1024 * 1024); // 50 files or 200MB
    /// assert_eq!(config.max_trajectories, Some(50));
    /// assert_eq!(config.total_size_limit, Some(200 * 1024 * 1024));
    /// ```
    pub fn with_limits(max_trajectories: usize, total_size_limit: u64) -> Self {
        Self {
            max_trajectories: Some(max_trajectories),
            total_size_limit: Some(total_size_limit),
        }
    }
}

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
    ///
    /// Creates a basic file storage that saves trajectories as uncompressed JSON
    /// files with no automatic rotation.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path for trajectory files, or a single file path
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Parent directory cannot be created
    /// - Path is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::trajectory::storage::FileStorage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Directory mode - saves multiple files
    /// let storage = FileStorage::new("trajectories")?;
    ///
    /// // Single file mode
    /// let storage = FileStorage::new("trajectory.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<P: AsRef<Path>>(path: P) -> SageResult<Self> {
        Self::with_config(path, false, RotationConfig::default())
    }

    /// Create a new file storage with optional compression
    ///
    /// # Arguments
    /// * `path` - Directory path for trajectory files
    /// * `enable_compression` - Whether to compress files with gzip
    ///
    /// # Compression Benefits
    /// - Reduces file sizes by 5-10x for typical trajectory files
    /// - Saves significant disk space for large-scale deployments
    /// - Transparent - loading works with both compressed and uncompressed files
    ///
    /// # Example
    /// ```no_run
    /// use sage_core::trajectory::storage::FileStorage;
    /// # use sage_core::error::SageResult;
    /// # fn example() -> SageResult<()> {
    /// // Enable compression for production
    /// let storage = FileStorage::with_compression("trajectories", true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_compression<P: AsRef<Path>>(path: P, enable_compression: bool) -> SageResult<Self> {
        Self::with_config(path, enable_compression, RotationConfig::default())
    }

    /// Create a new file storage with compression and rotation configuration
    ///
    /// # Arguments
    /// * `path` - Directory path for trajectory files
    /// * `enable_compression` - Whether to compress files with gzip
    /// * `rotation_config` - Configuration for file rotation
    ///
    /// # Example
    /// ```no_run
    /// use sage_core::trajectory::storage::{FileStorage, RotationConfig};
    /// # use sage_core::error::SageResult;
    /// # fn example() -> SageResult<()> {
    /// // Keep at most 10 trajectories
    /// let rotation = RotationConfig::with_max_trajectories(10);
    /// let storage = FileStorage::with_config("trajectories", true, rotation)?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Get the file path for a trajectory ID.
    ///
    /// In directory mode, generates a path with the trajectory ID.
    /// In single-file mode, returns the base path.
    fn get_file_path(&self, id: Id) -> PathBuf {
        if self.is_directory_path() {
            self.base_path.join(format!("{}.json", id))
        } else {
            // If base_path is a file, use it directly for single trajectory
            self.base_path.clone()
        }
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

    /// Determine if base_path should be treated as a directory.
    ///
    /// A path is considered a directory if:
    /// 1. It exists and is a directory, OR
    /// 2. It doesn't exist but has no file extension (assumed to be a directory)
    ///
    /// This allows the storage to work in two modes:
    /// - Directory mode: Multiple trajectory files with timestamp-based names
    /// - File mode: Single trajectory file that gets overwritten
    fn is_directory_path(&self) -> bool {
        if self.base_path.exists() {
            self.base_path.is_dir()
        } else {
            // If path doesn't exist, check if it has an extension
            // Paths without extensions are assumed to be directories
            self.base_path.extension().is_none()
        }
    }

    /// Save a trajectory record with gzip compression
    ///
    /// Compresses the trajectory JSON using gzip before writing to disk. This can reduce
    /// file sizes by 5-10x for typical trajectory files, significantly reducing disk space usage.
    ///
    /// # File Format
    /// - Compressed files use the `.json.gz` extension
    /// - Uses gzip default compression level (6)
    /// - Can be decompressed with standard gzip tools
    ///
    /// # Example
    /// ```no_run
    /// use sage_core::trajectory::storage::FileStorage;
    /// # use sage_core::error::SageResult;
    /// # async fn example() -> SageResult<()> {
    /// let storage = FileStorage::with_compression("trajectories", true)?;
    /// // All saves will automatically use compression
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, record), fields(id = %record.id, compressed = true))]
    pub async fn save_compressed(&self, record: &TrajectoryRecord) -> SageResult<()> {
        let file_path = if self.is_directory_path() {
            // If base_path is a directory, generate a new filename with .json.gz extension
            // Use millisecond precision to avoid filename collisions
            self.base_path.join(format!(
                "sage_{}.json.gz",
                chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f")
            ))
        } else {
            // If base_path is a file, add .gz extension if not present
            let path = self.base_path.clone();
            if path.extension().and_then(|s| s.to_str()) == Some("gz") {
                path
            } else {
                path.with_extension("json.gz")
            }
        };

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context(format!("Failed to create trajectory directory: {:?}", parent))?;
        }

        // Serialize record
        let json = serde_json::to_string_pretty(record)
            .context("Failed to serialize trajectory record")?;

        // Compress using gzip
        let compressed = tokio::task::spawn_blocking(move || {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(json.as_bytes())?;
            encoder.finish().map_err(|e| {
                SageError::io(format!("Failed to compress trajectory: {}", e))
            })
        })
        .await
        .context("Compression task failed")??;

        // Write compressed data to file
        fs::write(&file_path, &compressed)
            .await
            .context(format!("Failed to write compressed trajectory to {:?}", file_path))?;

        tracing::info!(
            compressed_size = compressed.len(),
            path = %file_path.display(),
            "trajectory saved with compression"
        );

        Ok(())
    }

    /// Load a trajectory record with automatic compression detection
    ///
    /// Automatically detects and handles both compressed (.json.gz) and uncompressed (.json)
    /// trajectory files. This allows seamless migration between compressed and uncompressed
    /// storage without code changes.
    ///
    /// # Detection
    /// - Files with `.gz` extension are decompressed using gzip
    /// - Files with `.json` extension are read as plain text
    /// - Detection is based on file extension, not content
    ///
    /// # Note
    /// This searches through all trajectory files to find the one with matching ID,
    /// since filenames are timestamp-based, not ID-based.
    ///
    /// # Example
    /// ```no_run
    /// use sage_core::trajectory::storage::FileStorage;
    /// # use sage_core::error::SageResult;
    /// # async fn example() -> SageResult<()> {
    /// let storage = FileStorage::new("trajectories")?;
    /// // Works with both .json and .json.gz files
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self), fields(id = %id))]
    pub async fn load_compressed(&self, id: Id) -> SageResult<Option<TrajectoryRecord>> {
        if self.is_directory_path() {
            // Directory mode: Scan through all files to find the one with matching ID
            if !self.base_path.exists() {
                return Ok(None);
            }

            let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
                SageError::config(format!(
                    "Failed to read trajectory directory {:?}: {}",
                    self.base_path, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                SageError::config(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());

                let record_opt = if extension == Some("gz") {
                    // Compressed file
                    Self::load_gzip_file(&path).await.ok().flatten()
                } else if extension == Some("json") {
                    // Uncompressed file
                    if let Ok(content) = fs::read_to_string(&path).await {
                        serde_json::from_str::<TrajectoryRecord>(&content).ok()
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(record) = record_opt {
                    if record.id == id {
                        tracing::info!("trajectory loaded successfully");
                        return Ok(Some(record));
                    }
                }
            }

            tracing::debug!("trajectory not found");
            Ok(None)
        } else {
            // File mode: Check if the base_path itself exists and load it
            if self.base_path.exists() {
                if Self::is_compressed_file(&self.base_path) {
                    return Self::load_gzip_file(&self.base_path).await;
                } else {
                    let content = fs::read_to_string(&self.base_path)
                        .await
                        .context(format!("Failed to read trajectory from {:?}", self.base_path))?;

                    let record: TrajectoryRecord = serde_json::from_str(&content)
                        .context(format!("Failed to parse trajectory JSON from {:?}", self.base_path))?;

                    return Ok(Some(record));
                }
            }

            Ok(None)
        }
    }

    /// Load a gzip-compressed trajectory file.
    ///
    /// Internal method for loading and decompressing `.json.gz` files.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the compressed file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be read
    /// - Decompression fails
    /// - JSON parsing fails
    async fn load_gzip_file(path: &Path) -> SageResult<Option<TrajectoryRecord>> {
        let compressed = fs::read(path)
            .await
            .context(format!("Failed to read compressed trajectory from {:?}", path))?;

        let path_clone = path.to_path_buf();
        let content = tokio::task::spawn_blocking(move || {
            let mut decoder = GzDecoder::new(&compressed[..]);
            let mut decompressed = String::new();
            decoder.read_to_string(&mut decompressed).map_err(|e| {
                SageError::io(format!(
                    "Failed to decompress trajectory from {:?}: {}",
                    path_clone, e
                ))
            })?;
            Ok::<String, SageError>(decompressed)
        })
        .await
        .context("Decompression task failed")??;

        let record: TrajectoryRecord = serde_json::from_str(&content)
            .context(format!("Failed to parse trajectory JSON from {:?}", path))?;

        Ok(Some(record))
    }

    /// Check if a file is gzip-compressed based on extension.
    ///
    /// Returns `true` if the file has a `.gz` extension.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::FileStorage;
    /// use std::path::Path;
    ///
    /// assert!(FileStorage::is_compressed_file(Path::new("trajectory.json.gz")));
    /// assert!(FileStorage::is_compressed_file(Path::new("file.gz")));
    /// assert!(!FileStorage::is_compressed_file(Path::new("trajectory.json")));
    /// ```
    pub fn is_compressed_file(path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "gz")
            .unwrap_or(false)
    }

    /// Perform file rotation based on configured limits
    ///
    /// This method enforces trajectory storage limits by:
    /// 1. Deleting oldest files when max_trajectories is exceeded
    /// 2. Deleting oldest files when total_size_limit is exceeded
    ///
    /// Files are sorted by modification time, with oldest files deleted first.
    ///
    /// # Example
    /// ```no_run
    /// use sage_core::trajectory::storage::{FileStorage, RotationConfig};
    /// # use sage_core::error::SageResult;
    /// # async fn example() -> SageResult<()> {
    /// let rotation = RotationConfig::with_max_trajectories(10);
    /// let storage = FileStorage::with_config("trajectories", true, rotation)?;
    /// // Rotation happens automatically after save
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rotate_files(&self) -> SageResult<()> {
        // Only perform rotation if we're using directory mode
        if !self.is_directory_path() {
            return Ok(());
        }

        // If no rotation limits are set, nothing to do
        if self.rotation_config.max_trajectories.is_none()
            && self.rotation_config.total_size_limit.is_none()
        {
            return Ok(());
        }

        if !self.base_path.exists() {
            return Ok(());
        }

        // Collect all trajectory files with metadata
        #[derive(Debug)]
        struct FileInfo {
            path: PathBuf,
            size: u64,
            modified: std::time::SystemTime,
        }

        let mut files = Vec::new();
        let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
            SageError::config(format!(
                "Failed to read trajectory directory {:?}: {}",
                self.base_path, e
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            SageError::config(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            let extension = path.extension().and_then(|s| s.to_str());

            // Only consider .json and .gz files
            if extension == Some("json") || extension == Some("gz") {
                if let Ok(metadata) = fs::metadata(&path).await {
                    if let Ok(modified) = metadata.modified() {
                        files.push(FileInfo {
                            path: path.clone(),
                            size: metadata.len(),
                            modified,
                        });
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        files.sort_by_key(|f| f.modified);

        // Apply max_trajectories limit
        if let Some(max_trajectories) = self.rotation_config.max_trajectories {
            while files.len() > max_trajectories {
                if let Some(oldest) = files.first() {
                    tracing::info!(
                        "Rotating trajectory file (max count): {}",
                        oldest.path.display()
                    );
                    fs::remove_file(&oldest.path).await.map_err(|e| {
                        SageError::io(format!(
                            "Failed to delete trajectory file {:?}: {}",
                            oldest.path, e
                        ))
                    })?;
                    files.remove(0);
                }
            }
        }

        // Apply total_size_limit
        if let Some(size_limit) = self.rotation_config.total_size_limit {
            let mut total_size: u64 = files.iter().map(|f| f.size).sum();

            while total_size > size_limit && !files.is_empty() {
                if let Some(oldest) = files.first() {
                    tracing::info!(
                        "Rotating trajectory file (size limit): {} ({} bytes)",
                        oldest.path.display(),
                        oldest.size
                    );
                    total_size = total_size.saturating_sub(oldest.size);
                    fs::remove_file(&oldest.path).await.map_err(|e| {
                        SageError::io(format!(
                            "Failed to delete trajectory file {:?}: {}",
                            oldest.path, e
                        ))
                    })?;
                    files.remove(0);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl TrajectoryStorage for FileStorage {
    #[instrument(skip(self, record), fields(id = %record.id, compression = %self.enable_compression))]
    async fn save(&self, record: &TrajectoryRecord) -> SageResult<()> {
        // Use compression if enabled in configuration
        if self.enable_compression {
            self.save_compressed(record).await?;
        } else {
            let file_path = if self.is_directory_path() {
                // If base_path is a directory, generate a new filename
                // Use millisecond precision to avoid filename collisions
                self.base_path.join(format!(
                    "sage_{}.json",
                    chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f")
                ))
            } else {
                // If base_path is a file, use it directly
                self.base_path.clone()
            };

            // Ensure parent directory exists
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .context(format!("Failed to create trajectory directory: {:?}", parent))?;
            }

            // Serialize record
            let json = serde_json::to_string_pretty(record)
                .context("Failed to serialize trajectory record")?;

            // Write to file
            fs::write(&file_path, &json)
                .await
                .context(format!("Failed to write trajectory to {:?}", file_path))?;

            tracing::info!(
                size = json.len(),
                path = %file_path.display(),
                "trajectory saved without compression"
            );
        }

        // Perform rotation after saving
        self.rotate_files().await?;

        Ok(())
    }

    #[instrument(skip(self), fields(id = %id))]
    async fn load(&self, id: Id) -> SageResult<Option<TrajectoryRecord>> {
        // If compression is enabled, use load_compressed which handles both formats
        if self.enable_compression {
            return self.load_compressed(id).await;
        }

        if self.is_directory_path() {
            // Directory mode: Scan through all files to find the one with matching ID
            if !self.base_path.exists() {
                return Ok(None);
            }

            let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
                SageError::config(format!(
                    "Failed to read trajectory directory {:?}: {}",
                    self.base_path, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                SageError::config(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());

                if extension == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(record) = serde_json::from_str::<TrajectoryRecord>(&content) {
                            if record.id == id {
                                tracing::info!("trajectory loaded successfully");
                                return Ok(Some(record));
                            }
                        }
                    }
                }
            }

            Ok(None)
        } else {
            // File mode: Try to load directly from base_path
            let file_path = self.get_file_path(id);

            if !file_path.exists() {
                return Ok(None);
            }

            let content = fs::read_to_string(&file_path)
                .await
                .context(format!("Failed to read trajectory from {:?}", file_path))?;

            let record: TrajectoryRecord = serde_json::from_str(&content)
                .context(format!("Failed to parse trajectory JSON from {:?}", file_path))?;

            tracing::info!("trajectory loaded successfully");
            Ok(Some(record))
        }
    }

    #[instrument(skip(self))]
    async fn list(&self) -> SageResult<Vec<Id>> {
        let mut ids = Vec::new();

        // If base_path is a directory, scan for .json and .json.gz files
        if self.is_directory_path() {
            let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
                SageError::config(format!(
                    "Failed to read trajectory directory {:?}: {}",
                    self.base_path, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                SageError::config(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());

                // Handle both .json and .json.gz files
                if extension == Some("json") {
                    // Uncompressed JSON file
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(record) = serde_json::from_str::<TrajectoryRecord>(&content) {
                            ids.push(record.id);
                        }
                    }
                } else if extension == Some("gz") {
                    // Compressed file (.json.gz)
                    if let Ok(Some(record)) = Self::load_gzip_file(&path).await {
                        ids.push(record.id);
                    }
                }
            }
        } else if self.base_path.exists() {
            // If base_path is a file, try to load it (compressed or uncompressed)
            if Self::is_compressed_file(&self.base_path) {
                if let Ok(Some(record)) = Self::load_gzip_file(&self.base_path).await {
                    ids.push(record.id);
                }
            } else if let Ok(content) = fs::read_to_string(&self.base_path).await {
                if let Ok(record) = serde_json::from_str::<TrajectoryRecord>(&content) {
                    ids.push(record.id);
                }
            }
        }

        Ok(ids)
    }

    async fn delete(&self, id: Id) -> SageResult<()> {
        if self.is_directory_path() {
            // Directory mode: Search for the file with matching ID and delete it
            if !self.base_path.exists() {
                return Err(SageError::config(format!(
                    "Trajectory {} not found",
                    id
                )));
            }

            let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
                SageError::config(format!(
                    "Failed to read trajectory directory {:?}: {}",
                    self.base_path, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                SageError::config(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());

                let record_opt = if extension == Some("gz") {
                    // Compressed file
                    Self::load_gzip_file(&path).await.ok().flatten()
                } else if extension == Some("json") {
                    // Uncompressed file
                    if let Ok(content) = fs::read_to_string(&path).await {
                        serde_json::from_str::<TrajectoryRecord>(&content).ok()
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(record) = record_opt {
                    if record.id == id {
                        // Found the file, delete it
                        fs::remove_file(&path).await
                            .context(format!("failed to delete trajectory file '{}'", path.display()))?;
                        return Ok(());
                    }
                }
            }

            // File not found
            Err(SageError::config(format!(
                "Trajectory {} not found",
                id
            )))
        } else {
            // File mode: Just delete the base_path if it exists
            if self.base_path.exists() {
                fs::remove_file(&self.base_path).await
                    .context(format!("failed to delete trajectory file '{}'", self.base_path.display()))?;
                Ok(())
            } else {
                Err(SageError::config(format!(
                    "Trajectory {} not found",
                    id
                )))
            }
        }
    }

    async fn statistics(&self) -> SageResult<StorageStatistics> {
        let mut total_records = 0;
        let mut total_size = 0u64;

        if self.is_directory_path() {
            if !self.base_path.exists() {
                return Ok(StorageStatistics {
                    total_records: 0,
                    total_size_bytes: 0,
                    average_record_size: 0,
                });
            }

            // Iterate through all files in the directory
            let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
                SageError::config(format!(
                    "Failed to read trajectory directory {:?}: {}",
                    self.base_path, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                SageError::config(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());

                // Only count .json and .gz files
                if extension == Some("json") || extension == Some("gz") {
                    if let Ok(metadata) = fs::metadata(&path).await {
                        total_size += metadata.len();
                        total_records += 1;
                    }
                }
            }
        } else {
            // File mode: Just check the single file
            if self.base_path.exists() {
                if let Ok(metadata) = fs::metadata(&self.base_path).await {
                    total_size = metadata.len();
                    total_records = 1;
                }
            }
        }

        let average_size = if total_records == 0 {
            0
        } else {
            total_size / total_records as u64
        };

        Ok(StorageStatistics {
            total_records,
            total_size_bytes: total_size,
            average_record_size: average_size,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// In-memory trajectory storage.
///
/// Stores trajectory records in memory using a HashMap. Useful for testing
/// and temporary storage scenarios where persistence is not required.
///
/// # Examples
///
/// ```no_run
/// use sage_core::trajectory::storage::{TrajectoryStorage, MemoryStorage};
/// use sage_core::trajectory::recorder::TrajectoryRecord;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = MemoryStorage::new();
///
/// // Save a trajectory
/// # let record = TrajectoryRecord {
/// #     id: uuid::Uuid::new_v4(),
/// #     task: "example".to_string(),
/// #     start_time: "2024-01-01T00:00:00Z".to_string(),
/// #     end_time: "2024-01-01T00:05:00Z".to_string(),
/// #     provider: "test".to_string(),
/// #     model: "test".to_string(),
/// #     max_steps: Some(10),
/// #     llm_interactions: vec![],
/// #     agent_steps: vec![],
/// #     success: true,
/// #     final_result: Some("done".to_string()),
/// #     execution_time: 5.0,
/// # };
/// storage.save(&record).await?;
///
/// // List all trajectories
/// let ids = storage.list().await?;
/// println!("Stored {} trajectories in memory", ids.len());
/// # Ok(())
/// # }
/// ```
pub struct MemoryStorage {
    records: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<Id, TrajectoryRecord>>>,
}

impl MemoryStorage {
    /// Create a new memory storage.
    ///
    /// Initializes an empty in-memory storage with no persisted data.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::MemoryStorage;
    ///
    /// let storage = MemoryStorage::new();
    /// // or use Default
    /// let storage = MemoryStorage::default();
    /// ```
    pub fn new() -> Self {
        Self {
            records: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TrajectoryStorage for MemoryStorage {
    async fn save(&self, record: &TrajectoryRecord) -> SageResult<()> {
        // TODO: Fix after trajectory record refactor - need to generate ID
        let mut records = self.records.lock().await;
        let id = uuid::Uuid::new_v4();
        records.insert(id, record.clone());
        Ok(())
    }

    async fn load(&self, id: Id) -> SageResult<Option<TrajectoryRecord>> {
        let records = self.records.lock().await;
        Ok(records.get(&id).cloned())
    }

    async fn list(&self) -> SageResult<Vec<Id>> {
        let records = self.records.lock().await;
        Ok(records.keys().cloned().collect())
    }

    async fn delete(&self, id: Id) -> SageResult<()> {
        let mut records = self.records.lock().await;
        records.remove(&id);
        Ok(())
    }

    async fn statistics(&self) -> SageResult<StorageStatistics> {
        let records = self.records.lock().await;
        let total_records = records.len();

        // Estimate size by serializing all records
        let mut total_size = 0u64;
        for record in records.values() {
            if let Ok(json) = serde_json::to_string(record) {
                total_size += json.len() as u64;
            }
        }

        let average_size = if total_records == 0 {
            0
        } else {
            total_size / total_records as u64
        };

        Ok(StorageStatistics {
            total_records,
            total_size_bytes: total_size,
            average_record_size: average_size,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod compression_tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper function to create a sample trajectory record
    fn create_test_record() -> TrajectoryRecord {
        use crate::trajectory::recorder::{
            AgentStepRecord, LLMInteractionRecord, LLMResponseRecord, TokenUsageRecord,
        };

        TrajectoryRecord {
            id: uuid::Uuid::new_v4(),
            task: "Test task".to_string(),
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: "2024-01-01T00:05:00Z".to_string(),
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            max_steps: Some(10),
            llm_interactions: vec![LLMInteractionRecord {
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                provider: "test-provider".to_string(),
                model: "test-model".to_string(),
                input_messages: vec![serde_json::json!({"role": "user", "content": "test"})],
                response: LLMResponseRecord {
                    content: "Test response".to_string(),
                    model: Some("test-model".to_string()),
                    finish_reason: Some("stop".to_string()),
                    usage: Some(TokenUsageRecord {
                        input_tokens: 10,
                        output_tokens: 20,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                        reasoning_tokens: None,
                    }),
                    tool_calls: None,
                },
                tools_available: Some(vec!["tool1".to_string(), "tool2".to_string()]),
            }],
            agent_steps: vec![AgentStepRecord {
                step_number: 1,
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                state: "Running".to_string(),
                llm_messages: Some(vec![serde_json::json!({"role": "user", "content": "test"})]),
                llm_response: Some(LLMResponseRecord {
                    content: "Test response".to_string(),
                    model: Some("test-model".to_string()),
                    finish_reason: Some("stop".to_string()),
                    usage: Some(TokenUsageRecord {
                        input_tokens: 10,
                        output_tokens: 20,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                        reasoning_tokens: None,
                    }),
                    tool_calls: None,
                }),
                tool_calls: None,
                tool_results: None,
                reflection: Some("Test reflection".to_string()),
                error: None,
            }],
            success: true,
            final_result: Some("Test completed".to_string()),
            execution_time: 300.0,
        }
    }

    #[tokio::test]
    async fn test_save_and_load_compressed() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test_trajectory.json.gz");
        let storage = FileStorage::new(&storage_path).unwrap();

        // Create and save a compressed trajectory
        let record = create_test_record();
        let record_id = record.id;

        storage.save_compressed(&record).await.unwrap();

        // Verify file exists and is compressed
        assert!(storage_path.exists());
        assert!(FileStorage::is_compressed_file(&storage_path));

        // Load the compressed trajectory
        let loaded = storage.load_compressed(record_id).await.unwrap();
        assert!(loaded.is_some());

        let loaded_record = loaded.unwrap();
        assert_eq!(loaded_record.id, record_id);
        assert_eq!(loaded_record.task, "Test task");
        assert_eq!(loaded_record.success, true);
        assert_eq!(loaded_record.agent_steps.len(), 1);
    }

    #[tokio::test]
    async fn test_load_compressed_with_auto_detection() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Save compressed trajectory
        let record = create_test_record();
        let record_id = record.id;

        storage.save_compressed(&record).await.unwrap();

        // load_compressed should automatically detect the .json.gz file
        let loaded = storage.load_compressed(record_id).await.unwrap();
        assert!(loaded.is_some());

        let loaded_record = loaded.unwrap();
        assert_eq!(loaded_record.id, record_id);
    }

    #[tokio::test]
    async fn test_load_compressed_fallback_to_uncompressed() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Save uncompressed trajectory using regular save
        let record = create_test_record();
        let record_id = record.id;

        storage.save(&record).await.unwrap();

        // load_compressed should fall back to reading the uncompressed .json file
        let loaded = storage.load_compressed(record_id).await.unwrap();
        assert!(loaded.is_some());

        let loaded_record = loaded.unwrap();
        assert_eq!(loaded_record.id, record_id);
    }

    #[tokio::test]
    async fn test_compression_reduces_file_size() {
        let temp_dir = TempDir::new().unwrap();

        // Create uncompressed file
        let uncompressed_path = temp_dir.path().join("uncompressed.json");
        let storage_uncompressed = FileStorage::new(&uncompressed_path).unwrap();

        // Create compressed file
        let compressed_path = temp_dir.path().join("compressed.json.gz");
        let storage_compressed = FileStorage::new(&compressed_path).unwrap();

        // Save the same record in both formats
        let record = create_test_record();
        storage_uncompressed.save(&record).await.unwrap();
        storage_compressed.save_compressed(&record).await.unwrap();

        // Check file sizes
        let uncompressed_size = fs::metadata(&uncompressed_path).await.unwrap().len();
        let compressed_size = fs::metadata(&compressed_path).await.unwrap().len();

        // Compressed should be smaller (with reasonable test data, typically 5-10x smaller)
        assert!(compressed_size < uncompressed_size);
        println!(
            "Uncompressed: {} bytes, Compressed: {} bytes, Ratio: {:.2}x",
            uncompressed_size,
            compressed_size,
            uncompressed_size as f64 / compressed_size as f64
        );
    }

    #[tokio::test]
    async fn test_list_includes_compressed_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Save one compressed and one uncompressed trajectory
        let record1 = create_test_record();
        let record2 = create_test_record();

        storage.save_compressed(&record1).await.unwrap();
        storage.save(&record2).await.unwrap();

        // List should include both files
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&record1.id));
        assert!(ids.contains(&record2.id));
    }

    #[tokio::test]
    async fn test_delete_compressed_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Save compressed trajectory
        let record = create_test_record();
        let record_id = record.id;

        storage.save_compressed(&record).await.unwrap();

        // Verify file exists
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 1);

        // Delete the trajectory
        storage.delete(record_id).await.unwrap();

        // Verify file is deleted
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 0);
    }

    #[tokio::test]
    async fn test_statistics_includes_compressed_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Save compressed and uncompressed trajectories
        let record1 = create_test_record();
        let record2 = create_test_record();

        storage.save_compressed(&record1).await.unwrap();
        storage.save(&record2).await.unwrap();

        // Get statistics
        let stats = storage.statistics().await.unwrap();

        assert_eq!(stats.total_records, 2);
        assert!(stats.total_size_bytes > 0);
        assert!(stats.average_record_size > 0);
    }

    #[tokio::test]
    async fn test_is_compressed_file() {
        assert!(FileStorage::is_compressed_file(Path::new("file.json.gz")));
        assert!(FileStorage::is_compressed_file(Path::new("file.gz")));
        assert!(!FileStorage::is_compressed_file(Path::new("file.json")));
        assert!(!FileStorage::is_compressed_file(Path::new("file.txt")));
        assert!(!FileStorage::is_compressed_file(Path::new("file")));
    }

    #[tokio::test]
    async fn test_load_nonexistent_compressed_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Try to load a non-existent trajectory
        let fake_id = uuid::Uuid::new_v4();
        let result = storage.load_compressed(fake_id).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_with_compression_config_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage with compression enabled
        let storage = FileStorage::with_compression(&storage_dir, true).unwrap();

        // Save a record using the trait's save() method
        let record = create_test_record();
        let record_id = record.id;

        storage.save(&record).await.unwrap();

        // Verify that a compressed file was created
        let mut entries = fs::read_dir(&storage_dir).await.unwrap();
        let mut found_gz = false;

        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("gz") {
                found_gz = true;
                break;
            }
        }

        assert!(found_gz, "Expected to find a .gz file when compression is enabled");

        // Load the record back
        let loaded = storage.load(record_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, record_id);
    }

    #[tokio::test]
    async fn test_with_compression_config_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage with compression disabled
        let storage = FileStorage::with_compression(&storage_dir, false).unwrap();

        // Save a record using the trait's save() method
        let record = create_test_record();
        let record_id = record.id;

        storage.save(&record).await.unwrap();

        // Verify that an uncompressed JSON file was created (not .gz)
        let mut entries = fs::read_dir(&storage_dir).await.unwrap();
        let mut found_json = false;

        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str());
            if ext == Some("json") && !path.to_str().unwrap().ends_with(".gz") {
                found_json = true;
                break;
            }
        }

        assert!(found_json, "Expected to find a .json file when compression is disabled");

        // Load the record back
        let loaded = storage.load(record_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, record_id);
    }

    #[tokio::test]
    async fn test_new_defaults_to_no_compression() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage with new() which should default to no compression
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Save a record
        let record = create_test_record();

        storage.save(&record).await.unwrap();

        // Verify that an uncompressed JSON file was created
        let mut entries = fs::read_dir(&storage_dir).await.unwrap();
        let mut found_json = false;

        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str());
            if ext == Some("json") && !path.to_str().unwrap().ends_with(".gz") {
                found_json = true;
                break;
            }
        }

        assert!(found_json, "Expected new() to default to no compression");
    }
}

#[cfg(test)]
mod rotation_tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper function to create a sample trajectory record
    fn create_test_record() -> TrajectoryRecord {
        use crate::trajectory::recorder::{
            AgentStepRecord, LLMInteractionRecord, LLMResponseRecord, TokenUsageRecord,
        };

        TrajectoryRecord {
            id: uuid::Uuid::new_v4(),
            task: "Test task".to_string(),
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: "2024-01-01T00:05:00Z".to_string(),
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            max_steps: Some(10),
            llm_interactions: vec![LLMInteractionRecord {
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                provider: "test-provider".to_string(),
                model: "test-model".to_string(),
                input_messages: vec![serde_json::json!({"role": "user", "content": "test"})],
                response: LLMResponseRecord {
                    content: "Test response".to_string(),
                    model: Some("test-model".to_string()),
                    finish_reason: Some("stop".to_string()),
                    usage: Some(TokenUsageRecord {
                        input_tokens: 10,
                        output_tokens: 20,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                        reasoning_tokens: None,
                    }),
                    tool_calls: None,
                },
                tools_available: Some(vec!["tool1".to_string(), "tool2".to_string()]),
            }],
            agent_steps: vec![AgentStepRecord {
                step_number: 1,
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                state: "Running".to_string(),
                llm_messages: Some(vec![serde_json::json!({"role": "user", "content": "test"})]),
                llm_response: Some(LLMResponseRecord {
                    content: "Test response".to_string(),
                    model: Some("test-model".to_string()),
                    finish_reason: Some("stop".to_string()),
                    usage: Some(TokenUsageRecord {
                        input_tokens: 10,
                        output_tokens: 20,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                        reasoning_tokens: None,
                    }),
                    tool_calls: None,
                }),
                tool_calls: None,
                tool_results: None,
                reflection: Some("Test reflection".to_string()),
                error: None,
            }],
            success: true,
            final_result: Some("Test completed".to_string()),
            execution_time: 300.0,
        }
    }

    #[tokio::test]
    async fn test_rotation_max_trajectories() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage with max 3 trajectories
        let rotation = RotationConfig::with_max_trajectories(3);
        let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

        // Save 5 trajectories
        for _ in 0..5 {
            let record = create_test_record();
            storage.save(&record).await.unwrap();
            // Small delay to ensure different modification times
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Should only have 3 files
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 3, "Should only keep 3 trajectories");
    }

    #[tokio::test]
    async fn test_rotation_total_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // First, create a test file to determine approximate size
        let test_storage = FileStorage::new(&storage_dir).unwrap();
        let test_record = create_test_record();
        test_storage.save(&test_record).await.unwrap();

        let stats = test_storage.statistics().await.unwrap();
        let file_size = stats.average_record_size;

        // Clean up test file
        test_storage.delete(test_record.id).await.unwrap();

        // Create storage with size limit for ~2.5 files
        let size_limit = file_size * 5 / 2;
        let rotation = RotationConfig::with_total_size_limit(size_limit);
        let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

        // Save 5 trajectories
        for _ in 0..5 {
            let record = create_test_record();
            storage.save(&record).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Should keep only 2 files (since we set limit to ~2.5 files)
        let stats = storage.statistics().await.unwrap();
        assert!(
            stats.total_records <= 2,
            "Should keep at most 2 trajectories based on size limit"
        );
        assert!(
            stats.total_size_bytes <= size_limit,
            "Total size should be within limit"
        );
    }

    #[tokio::test]
    async fn test_rotation_with_both_limits() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage with both limits
        let rotation = RotationConfig::with_limits(5, 1024 * 1024); // 5 files, 1MB
        let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

        // Save 10 trajectories
        for _ in 0..10 {
            let record = create_test_record();
            storage.save(&record).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Should only have 5 files (limited by max_trajectories)
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 5, "Should only keep 5 trajectories");
    }

    #[tokio::test]
    async fn test_rotation_with_compression() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage with compression and max 3 trajectories
        let rotation = RotationConfig::with_max_trajectories(3);
        let storage = FileStorage::with_config(&storage_dir, true, rotation).unwrap();

        // Save 5 trajectories
        for _ in 0..5 {
            let record = create_test_record();
            storage.save(&record).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Should only have 3 compressed files
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 3, "Should only keep 3 compressed trajectories");

        // Verify files are compressed
        let mut entries = fs::read_dir(&storage_dir).await.unwrap();
        let mut gz_count = 0;

        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("gz") {
                gz_count += 1;
            }
        }

        assert_eq!(gz_count, 3, "Should have 3 .gz files");
    }

    #[tokio::test]
    async fn test_rotation_keeps_newest_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage with max 2 trajectories
        let rotation = RotationConfig::with_max_trajectories(2);
        let storage = FileStorage::with_config(&storage_dir, false, rotation).unwrap();

        // Save 3 trajectories and remember their IDs
        let mut ids = Vec::new();
        for _ in 0..3 {
            let record = create_test_record();
            ids.push(record.id);
            storage.save(&record).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Should only have the last 2 trajectories
        let remaining_ids = storage.list().await.unwrap();
        assert_eq!(remaining_ids.len(), 2);

        // The first ID should be deleted, last 2 should remain
        assert!(!remaining_ids.contains(&ids[0]), "Oldest should be deleted");
        assert!(remaining_ids.contains(&ids[1]), "Second should remain");
        assert!(remaining_ids.contains(&ids[2]), "Newest should remain");
    }

    #[tokio::test]
    async fn test_rotation_no_limits() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create storage without rotation limits
        let storage = FileStorage::new(&storage_dir).unwrap();

        // Save 5 trajectories
        for _ in 0..5 {
            let record = create_test_record();
            storage.save(&record).await.unwrap();
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        }

        // Should keep all 5 files
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 5, "Should keep all trajectories when no limits set");
    }

    #[tokio::test]
    async fn test_rotation_config_builders() {
        // Test with_max_trajectories
        let config1 = RotationConfig::with_max_trajectories(10);
        assert_eq!(config1.max_trajectories, Some(10));
        assert_eq!(config1.total_size_limit, None);

        // Test with_total_size_limit
        let config2 = RotationConfig::with_total_size_limit(1024);
        assert_eq!(config2.max_trajectories, None);
        assert_eq!(config2.total_size_limit, Some(1024));

        // Test with_limits
        let config3 = RotationConfig::with_limits(5, 2048);
        assert_eq!(config3.max_trajectories, Some(5));
        assert_eq!(config3.total_size_limit, Some(2048));

        // Test default
        let config4 = RotationConfig::default();
        assert_eq!(config4.max_trajectories, None);
        assert_eq!(config4.total_size_limit, None);
    }

    #[tokio::test]
    async fn test_rotation_does_not_affect_file_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("single_trajectory.json");

        // Create storage pointing to a single file with rotation config
        let rotation = RotationConfig::with_max_trajectories(1);
        let storage = FileStorage::with_config(&file_path, false, rotation).unwrap();

        // Save multiple times to the same file
        for _ in 0..3 {
            let record = create_test_record();
            storage.save(&record).await.unwrap();
        }

        // File should still exist (rotation shouldn't delete single file mode)
        assert!(file_path.exists());

        // Should only have 1 record (the file gets overwritten)
        let ids = storage.list().await.unwrap();
        assert_eq!(ids.len(), 1);
    }
}
