//! In-memory trajectory storage implementation

use crate::error::SageResult;
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use async_trait::async_trait;
use std::any::Any;

use super::trait_def::TrajectoryStorage;
use super::types::StorageStatistics;

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
