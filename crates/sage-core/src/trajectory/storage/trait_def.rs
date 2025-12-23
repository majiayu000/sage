//! Trajectory storage trait definition

use crate::error::SageResult;
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use async_trait::async_trait;
use std::any::Any;

use super::types::StorageStatistics;

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
