//! Write operations for messages and snapshots

use crate::error::{SageError, SageResult};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::debug;

use super::super::super::types::{FileHistorySnapshot, SessionId};
use crate::session::types::unified::SessionMessage;
use super::core::JsonlSessionStorage;

impl JsonlSessionStorage {
    /// Append a message to the session
    pub async fn append_message(
        &self,
        id: &SessionId,
        message: &SessionMessage,
    ) -> SageResult<()> {
        self.ensure_session_dir(id).await?;

        let path = self.messages_path(id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open messages file: {}", e)))?;

        let json = serde_json::to_string(message)
            .map_err(|e| SageError::json(format!("Failed to serialize message: {}", e)))?;

        let mut json_line = String::with_capacity(json.len() + 1);
        json_line.push_str(&json);
        json_line.push('\n');

        file.write_all(json_line.as_bytes())
            .await
            .map_err(|e| SageError::io(format!("Failed to write message: {}", e)))?;

        debug!("Appended message {} to session {}", message.uuid, id);
        Ok(())
    }

    /// Append a file history snapshot
    pub async fn append_snapshot(
        &self,
        id: &SessionId,
        snapshot: &FileHistorySnapshot,
    ) -> SageResult<()> {
        self.ensure_session_dir(id).await?;

        let path = self.snapshots_path(id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open snapshots file: {}", e)))?;

        let json = serde_json::to_string(snapshot)
            .map_err(|e| SageError::json(format!("Failed to serialize snapshot: {}", e)))?;

        let mut json_line = String::with_capacity(json.len() + 1);
        json_line.push_str(&json);
        json_line.push('\n');

        file.write_all(json_line.as_bytes())
            .await
            .map_err(|e| SageError::io(format!("Failed to write snapshot: {}", e)))?;

        debug!(
            "Appended snapshot for message {} to session {}",
            snapshot.message_id, id
        );
        Ok(())
    }
}
