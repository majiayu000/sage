//! Trajectory storage implementations

use crate::error::{SageError, SageResult};
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use async_trait::async_trait;
use std::any::Any;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Trait for trajectory storage backends
#[async_trait]
pub trait TrajectoryStorage: Send + Sync {
    /// Save a trajectory record
    async fn save(&self, record: &TrajectoryRecord) -> SageResult<()>;

    /// Load a trajectory record by ID
    async fn load(&self, id: Id) -> SageResult<Option<TrajectoryRecord>>;

    /// List all trajectory IDs
    async fn list(&self) -> SageResult<Vec<Id>>;

    /// Delete a trajectory record
    async fn delete(&self, id: Id) -> SageResult<()>;

    /// Get storage statistics
    async fn statistics(&self) -> SageResult<StorageStatistics>;

    /// For downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStatistics {
    /// Total number of stored trajectories
    pub total_records: usize,
    /// Total storage size in bytes
    pub total_size_bytes: u64,
    /// Average record size in bytes
    pub average_record_size: u64,
}

/// File-based trajectory storage
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    /// Create a new file storage
    pub fn new<P: AsRef<Path>>(path: P) -> SageResult<Self> {
        let base_path = path.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SageError::config(format!("Failed to create trajectory directory: {}", e))
            })?;
        }

        Ok(Self { base_path })
    }

    /// Get the file path for a trajectory ID
    fn get_file_path(&self, id: Id) -> PathBuf {
        if self.base_path.is_dir() {
            self.base_path.join(format!("{}.json", id))
        } else {
            // If base_path is a file, use it directly for single trajectory
            self.base_path.clone()
        }
    }

    /// Get the base path
    pub fn path(&self) -> &Path {
        &self.base_path
    }
}

#[async_trait]
impl TrajectoryStorage for FileStorage {
    async fn save(&self, record: &TrajectoryRecord) -> SageResult<()> {
        let file_path = if self.base_path.is_dir() {
            // If base_path is a directory, generate a new filename
            self.base_path.join(format!(
                "sage_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            ))
        } else {
            // If base_path is a file, use it directly
            self.base_path.clone()
        };

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Serialize record
        let json = serde_json::to_string_pretty(record)?;

        // Write to file
        fs::write(&file_path, json).await?;

        Ok(())
    }

    async fn load(&self, id: Id) -> SageResult<Option<TrajectoryRecord>> {
        let file_path = self.get_file_path(id);

        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path).await?;

        let record: TrajectoryRecord = serde_json::from_str(&content)?;

        Ok(Some(record))
    }

    async fn list(&self) -> SageResult<Vec<Id>> {
        // TODO: Fix after trajectory record refactor
        Ok(Vec::new())
    }

    async fn delete(&self, id: Id) -> SageResult<()> {
        let file_path = self.get_file_path(id);

        if file_path.exists() {
            fs::remove_file(&file_path).await?;
        }

        Ok(())
    }

    async fn statistics(&self) -> SageResult<StorageStatistics> {
        let ids = self.list().await?;
        let mut total_size = 0u64;

        for id in &ids {
            let file_path = self.get_file_path(*id);
            if let Ok(metadata) = fs::metadata(&file_path).await {
                total_size += metadata.len();
            }
        }

        let average_size = if ids.is_empty() {
            0
        } else {
            total_size / ids.len() as u64
        };

        Ok(StorageStatistics {
            total_records: ids.len(),
            total_size_bytes: total_size,
            average_record_size: average_size,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// In-memory trajectory storage (for testing)
pub struct MemoryStorage {
    records: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<Id, TrajectoryRecord>>>,
}

impl MemoryStorage {
    /// Create a new memory storage
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
