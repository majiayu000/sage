//! JSONL record types for append-only session persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::header::SessionId;
use super::message::SessionMessage;
use super::wire_token::WireTokenUsage;
use crate::session::file_tracking::FileHistorySnapshot;
use crate::session::types::base::SessionState;

/// JSONL record for real-time persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Sequence number (monotonically increasing)
    pub seq: u64,

    /// Record timestamp
    pub timestamp: DateTime<Utc>,

    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,

    /// Record payload (not flattened to avoid field conflicts)
    pub payload: SessionRecordPayload,
}

/// Record payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "recordType", rename_all = "snake_case")]
pub enum SessionRecordPayload {
    /// Message record
    Message(Box<SessionMessage>),
    /// File snapshot record
    Snapshot(FileHistorySnapshot),
    /// Metadata update record
    MetadataPatch(SessionMetadataPatch),
}

/// Metadata patch for incremental updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadataPatch {
    /// Update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// Updated message count
    #[serde(rename = "messageCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<usize>,

    /// Updated last prompt
    #[serde(rename = "lastPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_prompt: Option<String>,

    /// Updated summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Updated custom title
    #[serde(rename = "customTitle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,

    /// Updated state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<SessionState>,

    /// Updated token usage
    #[serde(rename = "tokenUsage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<WireTokenUsage>,
}
