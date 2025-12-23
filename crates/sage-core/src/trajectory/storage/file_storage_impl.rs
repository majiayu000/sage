//! TrajectoryStorage trait implementation for FileStorage

use crate::error::{ResultExt, SageError, SageResult};
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use async_trait::async_trait;
use std::any::Any;
use tokio::fs;
use tracing::instrument;

use super::file_storage::FileStorage;
use super::trait_def::TrajectoryStorage;
use super::types::StorageStatistics;

#[async_trait]
impl TrajectoryStorage for FileStorage {
    #[instrument(skip(self, record), fields(id = %record.id, compression = %self.enable_compression()))]
    async fn save(&self, record: &TrajectoryRecord) -> SageResult<()> {
        // Use compression if enabled in configuration
        if self.enable_compression() {
            self.save_compressed(record).await?;
        } else {
            self.save_uncompressed(record).await?;
        }

        // Perform rotation after saving
        self.rotate_files().await?;

        Ok(())
    }

    #[instrument(skip(self), fields(id = %id))]
    async fn load(&self, id: Id) -> SageResult<Option<TrajectoryRecord>> {
        // If compression is enabled, use load_compressed which handles both formats
        if self.enable_compression() {
            return self.load_compressed(id).await;
        }

        if self.is_directory_path() {
            self.load_uncompressed_directory(id).await
        } else {
            self.load_uncompressed_file(id).await
        }
    }

    #[instrument(skip(self))]
    async fn list(&self) -> SageResult<Vec<Id>> {
        let mut ids = Vec::new();

        // If base_path is a directory, scan for .json and .json.gz files
        if self.is_directory_path() {
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
        } else if self.base_path().exists() {
            // If base_path is a file, try to load it (compressed or uncompressed)
            if Self::is_compressed_file(self.base_path()) {
                if let Ok(Some(record)) = Self::load_gzip_file(self.base_path()).await {
                    ids.push(record.id);
                }
            } else if let Ok(content) = fs::read_to_string(self.base_path()).await {
                if let Ok(record) = serde_json::from_str::<TrajectoryRecord>(&content) {
                    ids.push(record.id);
                }
            }
        }

        Ok(ids)
    }

    async fn delete(&self, id: Id) -> SageResult<()> {
        if self.is_directory_path() {
            self.delete_from_directory(id).await
        } else {
            self.delete_single_file().await
        }
    }

    async fn statistics(&self) -> SageResult<StorageStatistics> {
        if self.is_directory_path() {
            self.statistics_directory().await
        } else {
            self.statistics_single_file().await
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl FileStorage {
    /// Delete trajectory from directory mode
    async fn delete_from_directory(&self, id: Id) -> SageResult<()> {
        if !self.base_path().exists() {
            return Err(SageError::config(format!("Trajectory {} not found", id)));
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
                Self::load_gzip_file(&path).await.ok().flatten()
            } else if extension == Some("json") {
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
                    fs::remove_file(&path).await.context(format!(
                        "failed to delete trajectory file '{}'",
                        path.display()
                    ))?;
                    return Ok(());
                }
            }
        }

        Err(SageError::config(format!("Trajectory {} not found", id)))
    }

    /// Delete single file
    async fn delete_single_file(&self) -> SageResult<()> {
        if self.base_path().exists() {
            fs::remove_file(self.base_path()).await.context(format!(
                "failed to delete trajectory file '{}'",
                self.base_path().display()
            ))?;
            Ok(())
        } else {
            Err(SageError::config("Trajectory not found".to_string()))
        }
    }

    /// Get statistics for directory mode
    async fn statistics_directory(&self) -> SageResult<StorageStatistics> {
        if !self.base_path().exists() {
            return Ok(StorageStatistics {
                total_records: 0,
                total_size_bytes: 0,
                average_record_size: 0,
            });
        }

        let mut total_records = 0;
        let mut total_size = 0u64;
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

            if extension == Some("json") || extension == Some("gz") {
                if let Ok(metadata) = fs::metadata(&path).await {
                    total_size += metadata.len();
                    total_records += 1;
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

    /// Get statistics for single file mode
    async fn statistics_single_file(&self) -> SageResult<StorageStatistics> {
        if self.base_path().exists() {
            if let Ok(metadata) = fs::metadata(self.base_path()).await {
                Ok(StorageStatistics {
                    total_records: 1,
                    total_size_bytes: metadata.len(),
                    average_record_size: metadata.len(),
                })
            } else {
                Ok(StorageStatistics {
                    total_records: 0,
                    total_size_bytes: 0,
                    average_record_size: 0,
                })
            }
        } else {
            Ok(StorageStatistics {
                total_records: 0,
                total_size_bytes: 0,
                average_record_size: 0,
            })
        }
    }
}
