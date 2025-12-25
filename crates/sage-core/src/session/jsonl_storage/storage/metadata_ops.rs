//! Metadata I/O operations

use crate::error::{SageError, SageResult};
use tokio::fs;
use tracing::debug;

use super::super::super::types::SessionId;
use super::super::metadata::SessionMetadata;
use super::core::JsonlSessionStorage;

impl JsonlSessionStorage {
    /// Save session metadata
    pub async fn save_metadata(
        &self,
        id: &SessionId,
        metadata: &SessionMetadata,
    ) -> SageResult<()> {
        let path = self.metadata_path(id);
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| SageError::json(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&path, json)
            .await
            .map_err(|e| SageError::io(format!("Failed to write metadata file: {}", e)))?;

        debug!("Saved metadata for session {}", id);
        Ok(())
    }

    /// Load session metadata
    pub async fn load_metadata(&self, id: &SessionId) -> SageResult<Option<SessionMetadata>> {
        let path = self.metadata_path(id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to read metadata file: {}", e)))?;

        let metadata: SessionMetadata = serde_json::from_str(&json)
            .map_err(|e| SageError::json(format!("Failed to deserialize metadata: {}", e)))?;

        Ok(Some(metadata))
    }
}
