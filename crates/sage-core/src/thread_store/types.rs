use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::runtime_protocol::RuntimeMessage;

use super::error::{ThreadStoreError, ThreadStoreResult};

pub type ThreadId = String;
pub type TurnId = String;
pub type ItemId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Active,
    Completed,
    Failed,
    Interrupted,
    Unknown,
}

impl ThreadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_store(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "interrupted" => Some(Self::Interrupted),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Started,
    Completed,
    Failed,
    Interrupted,
    Incomplete,
}

impl TurnStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
            Self::Incomplete => "incomplete",
        }
    }

    pub fn from_store(value: &str) -> Option<Self> {
        match value {
            "started" => Some(Self::Started),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "interrupted" => Some(Self::Interrupted),
            "incomplete" => Some(Self::Incomplete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteMode {
    MetadataOnly,
    MetadataAndPayloadFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacySourceKind {
    TrajectoryJsonl,
    SessionDirectory,
    SessionMessagesJsonl,
}

impl LegacySourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TrajectoryJsonl => "trajectory_jsonl",
            Self::SessionDirectory => "session_directory",
            Self::SessionMessagesJsonl => "session_messages_jsonl",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadRecord {
    pub thread_id: ThreadId,
    pub legacy_session_id: Option<String>,
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: ThreadStatus,
    pub archived_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub payload_deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, Value>,
}

impl ThreadRecord {
    pub fn new(thread_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            thread_id: thread_id.into(),
            legacy_session_id: None,
            title: None,
            cwd: None,
            provider: None,
            model: None,
            status: ThreadStatus::Active,
            archived_at: None,
            deleted_at: None,
            payload_deleted_at: None,
            created_at: now,
            updated_at: now,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRecord {
    pub turn_id: TurnId,
    pub thread_id: ThreadId,
    pub status: TurnStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sequence_start: Option<u64>,
    pub sequence_end: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadItemInput {
    pub item_id: Option<ItemId>,
    pub turn_id: Option<TurnId>,
    pub item_type: String,
    pub role: Option<String>,
    pub status: Option<String>,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub sequence: Option<u64>,
    pub legacy_uuid: Option<String>,
    pub payload_ref: Option<String>,
    pub payload_json: Option<Value>,
    pub search_text: Option<String>,
    pub partial_lineage: bool,
}

impl ThreadItemInput {
    pub fn new(item_type: impl Into<String>) -> Self {
        Self {
            item_id: None,
            turn_id: None,
            item_type: item_type.into(),
            role: None,
            status: None,
            source: "runtime".to_string(),
            created_at: Utc::now(),
            sequence: None,
            legacy_uuid: None,
            payload_ref: None,
            payload_json: None,
            search_text: None,
            partial_lineage: false,
        }
    }

    pub fn from_runtime_message(message: &RuntimeMessage) -> ThreadStoreResult<Self> {
        let value = serde_json::to_value(message)?;
        let envelope = value.as_object().ok_or_else(|| {
            ThreadStoreError::InvalidInput("runtime message is not an object".into())
        })?;
        let payload = envelope.get("payload").unwrap_or(&Value::Null);
        let message_type = envelope
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("runtime");
        let kind = envelope
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("runtime");
        let search_text = payload_text(payload);

        let item_type = payload
            .get("item_type")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| item_type_from_message(kind, message_type).to_string());

        let created_at = envelope
            .get("timestamp")
            .and_then(Value::as_str)
            .map(DateTime::parse_from_rfc3339)
            .transpose()?
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Ok(Self {
            item_id: envelope
                .get("item_id")
                .or_else(|| envelope.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string),
            turn_id: envelope
                .get("turn_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            item_type,
            role: payload
                .get("role")
                .and_then(Value::as_str)
                .map(str::to_string),
            status: payload
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_string),
            source: envelope
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("runtime")
                .to_string(),
            created_at,
            sequence: envelope.get("sequence").and_then(Value::as_u64),
            legacy_uuid: payload
                .get("legacy_uuid")
                .and_then(Value::as_str)
                .map(str::to_string),
            payload_ref: None,
            payload_json: Some(value),
            search_text,
            partial_lineage: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadItemRecord {
    pub item_id: ItemId,
    pub thread_id: ThreadId,
    pub turn_id: Option<TurnId>,
    pub item_type: String,
    pub role: Option<String>,
    pub status: Option<String>,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub sequence: u64,
    pub legacy_uuid: Option<String>,
    pub payload_ref: Option<String>,
    pub payload_json: Option<Value>,
    pub search_text: Option<String>,
    pub partial_lineage: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadLineage {
    pub thread_id: ThreadId,
    pub parent_thread_id: Option<ThreadId>,
    pub parent_turn_id: Option<TurnId>,
    pub parent_item_id: Option<ItemId>,
    pub fork_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadSnapshot {
    pub thread: ThreadRecord,
    pub lineage: Option<ThreadLineage>,
    pub turns: Vec<TurnRecord>,
    pub items: Vec<ThreadItemRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendResult {
    pub thread_id: ThreadId,
    pub turn_id: Option<TurnId>,
    pub item_id: ItemId,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadListQuery {
    pub include_archived: bool,
    pub limit: u64,
    pub offset: u64,
}

impl Default for ThreadListQuery {
    fn default() -> Self {
        Self {
            include_archived: false,
            limit: 50,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub include_archived: bool,
    pub limit: u64,
    pub offset: u64,
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            include_archived: false,
            limit: 50,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub thread: ThreadRecord,
    pub matched_item_id: Option<ItemId>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteResult {
    pub thread_id: ThreadId,
    pub metadata_deleted: bool,
    pub payload_files_deleted: usize,
    pub payload_delete_errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackfillOptions {
    pub source_kind: LegacySourceKind,
}

impl BackfillOptions {
    pub fn trajectory_jsonl() -> Self {
        Self {
            source_kind: LegacySourceKind::TrajectoryJsonl,
        }
    }

    pub fn session_directory() -> Self {
        Self {
            source_kind: LegacySourceKind::SessionDirectory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackfillReport {
    pub source_id: String,
    pub imported_threads: usize,
    pub imported_items: usize,
    pub errors: Vec<StoreErrorRecord>,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreErrorRecord {
    pub error_id: String,
    pub thread_id: Option<ThreadId>,
    pub source_id: Option<String>,
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub issues: Vec<RecoveryIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryIssue {
    pub code: RecoveryIssueCode,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryIssueCode {
    IncompleteTurn,
    CorruptLegacySource,
    MissingMetadata,
    SchemaVersionMismatch,
}

fn item_type_from_message(kind: &str, message_type: &str) -> &'static str {
    match (kind, message_type) {
        ("error", _) | (_, "error.reported") => "error",
        (_, "permission.requested" | "permission.resolved") => "permission",
        (_, "turn.started" | "turn.completed" | "turn.interrupted") => "turn",
        (_, "thread.started" | "thread.ended") => "thread",
        _ => "message",
    }
}

fn payload_text(payload: &Value) -> Option<String> {
    for key in ["content", "message", "result"] {
        if let Some(text) = payload.get(key).and_then(Value::as_str) {
            return Some(text.to_string());
        }
    }
    None
}
