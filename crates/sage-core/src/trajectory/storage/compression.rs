//! Compression utilities for trajectory storage

use crate::error::{ResultExt, SageError, SageResult};
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::{Read, Write};
use std::path::Path;
use tokio::fs;
use tracing::instrument;

use super::file_storage::FileStorage;

impl FileStorage {
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
            self.base_path().join(format!(
                "sage_{}.json.gz",
                chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f")
            ))
        } else {
            // If base_path is a file, add .gz extension if not present
            let path = self.base_path().to_path_buf();
            if path.extension().and_then(|s| s.to_str()) == Some("gz") {
                path
            } else {
                path.with_extension("json.gz")
            }
        };

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.context(format!(
                "Failed to create trajectory directory: {:?}",
                parent
            ))?;
        }

        // Serialize record
        let json = serde_json::to_string_pretty(record)
            .context("Failed to serialize trajectory record")?;

        // Compress using gzip
        let compressed = tokio::task::spawn_blocking(move || {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(json.as_bytes())?;
            encoder
                .finish()
                .map_err(|e| SageError::io(format!("Failed to compress trajectory: {}", e)))
        })
        .await
        .context("Compression task failed")??;

        // Write compressed data to file
        fs::write(&file_path, &compressed).await.context(format!(
            "Failed to write compressed trajectory to {:?}",
            file_path
        ))?;

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
            if !self.base_path().exists() {
                return Ok(None);
            }

            let mut entries = fs::read_dir(self.base_path()).await.map_err(|e| {
                SageError::config(format!(
                    "Failed to read trajectory directory {:?}: {}",
                    self.base_path(), e
                ))
            })?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| SageError::config(format!("Failed to read directory entry: {}", e)))?
            {
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
            if self.base_path().exists() {
                if Self::is_compressed_file(self.base_path()) {
                    return Self::load_gzip_file(self.base_path()).await;
                } else {
                    let content = fs::read_to_string(self.base_path()).await.context(format!(
                        "Failed to read trajectory from {:?}",
                        self.base_path()
                    ))?;

                    let record: TrajectoryRecord = serde_json::from_str(&content).context(
                        format!("Failed to parse trajectory JSON from {:?}", self.base_path()),
                    )?;

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
    pub(super) async fn load_gzip_file(path: &Path) -> SageResult<Option<TrajectoryRecord>> {
        let compressed = fs::read(path).await.context(format!(
            "Failed to read compressed trajectory from {:?}",
            path
        ))?;

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
}
