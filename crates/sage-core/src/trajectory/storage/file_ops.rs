//! File operations for FileStorage - save, load implementations

use crate::error::{ResultExt, SageError, SageResult};
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use tokio::fs;
use tracing::instrument;

use super::file_storage::FileStorage;

impl FileStorage {
    /// Save trajectory without compression
    #[instrument(skip(self, record), fields(id = %record.id))]
    pub(super) async fn save_uncompressed(&self, record: &TrajectoryRecord) -> SageResult<()> {
        let file_path = if self.is_directory_path() {
            // If base_path is a directory, generate a new filename
            // Use millisecond precision to avoid filename collisions
            self.base_path().join(format!(
                "sage_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f")
            ))
        } else {
            // If base_path is a file, use it directly
            self.base_path().to_path_buf()
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

        // Write to file
        fs::write(&file_path, &json)
            .await
            .context(format!("Failed to write trajectory to {:?}", file_path))?;

        tracing::info!(
            size = json.len(),
            path = %file_path.display(),
            "trajectory saved without compression"
        );

        Ok(())
    }

    /// Load trajectory from uncompressed file in directory mode
    #[instrument(skip(self), fields(id = %id))]
    pub(super) async fn load_uncompressed_directory(&self, id: Id) -> SageResult<Option<TrajectoryRecord>> {
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
    }

    /// Load trajectory from file mode (single file)
    #[instrument(skip(self), fields(id = %id))]
    pub(super) async fn load_uncompressed_file(&self, id: Id) -> SageResult<Option<TrajectoryRecord>> {
        let file_path = self.get_file_path(id);

        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path)
            .await
            .context(format!("Failed to read trajectory from {:?}", file_path))?;

        let record: TrajectoryRecord = serde_json::from_str(&content).context(format!(
            "Failed to parse trajectory JSON from {:?}",
            file_path
        ))?;

        tracing::info!("trajectory loaded successfully");
        Ok(Some(record))
    }
}
