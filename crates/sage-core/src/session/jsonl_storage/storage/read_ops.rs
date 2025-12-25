//! Read operations for messages and snapshots

use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, warn};

use super::super::super::types::{EnhancedMessage, FileHistorySnapshot, SessionId};
use super::core::JsonlSessionStorage;

impl JsonlSessionStorage {
    /// Load all messages from a session
    pub async fn load_messages(&self, id: &SessionId) -> SageResult<Vec<EnhancedMessage>> {
        let path = self.messages_path(id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open messages file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut messages = Vec::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| SageError::io(format!("Failed to read line: {}", e)))?
        {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<EnhancedMessage>(&line) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    warn!(
                        "Failed to parse message: {} - line: {}",
                        e,
                        &line[..50.min(line.len())]
                    );
                }
            }
        }

        debug!("Loaded {} messages from session {}", messages.len(), id);
        Ok(messages)
    }

    /// Load all snapshots from a session
    pub async fn load_snapshots(&self, id: &SessionId) -> SageResult<Vec<FileHistorySnapshot>> {
        let path = self.snapshots_path(id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open snapshots file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut snapshots = Vec::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| SageError::io(format!("Failed to read line: {}", e)))?
        {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<FileHistorySnapshot>(&line) {
                Ok(snapshot) => snapshots.push(snapshot),
                Err(e) => {
                    warn!(
                        "Failed to parse snapshot: {} - line: {}",
                        e,
                        &line[..50.min(line.len())]
                    );
                }
            }
        }

        debug!("Loaded {} snapshots from session {}", snapshots.len(), id);
        Ok(snapshots)
    }

    /// Get message by UUID
    pub async fn get_message(
        &self,
        session_id: &SessionId,
        message_uuid: &str,
    ) -> SageResult<Option<EnhancedMessage>> {
        let path = self.messages_path(session_id);

        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open messages file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| SageError::io(format!("Failed to read line: {}", e)))?
        {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<EnhancedMessage>(&line) {
                Ok(msg) => {
                    if msg.uuid == message_uuid {
                        return Ok(Some(msg));
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to parse message: {} - line: {}",
                        e,
                        &line[..50.min(line.len())]
                    );
                }
            }
        }

        Ok(None)
    }

    /// Get messages up to a specific UUID (for undo)
    pub async fn get_messages_until(
        &self,
        session_id: &SessionId,
        message_uuid: &str,
    ) -> SageResult<Vec<EnhancedMessage>> {
        let path = self.messages_path(session_id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open messages file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut result = Vec::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| SageError::io(format!("Failed to read line: {}", e)))?
        {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<EnhancedMessage>(&line) {
                Ok(msg) => {
                    let is_target = msg.uuid == message_uuid;
                    result.push(msg);
                    if is_target {
                        break;
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to parse message: {} - line: {}",
                        e,
                        &line[..50.min(line.len())]
                    );
                }
            }
        }

        Ok(result)
    }

    /// Get the message chain (following parentUuid links)
    pub async fn get_message_chain(
        &self,
        session_id: &SessionId,
        start_uuid: &str,
    ) -> SageResult<Vec<EnhancedMessage>> {
        let messages = self.load_messages(session_id).await?;

        // Build a map for quick lookup
        let msg_map: HashMap<&str, &EnhancedMessage> =
            messages.iter().map(|m| (m.uuid.as_str(), m)).collect();

        // Follow the chain from start
        let mut chain = Vec::new();
        let mut current_uuid = Some(start_uuid);

        while let Some(uuid) = current_uuid {
            if let Some(msg) = msg_map.get(uuid) {
                chain.push((*msg).clone());
                current_uuid = msg.parent_uuid.as_deref();
            } else {
                break;
            }
        }

        // Reverse to get chronological order
        chain.reverse();
        Ok(chain)
    }
}
