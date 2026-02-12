//! Session header and aggregate types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use super::message::SessionMessage;
use super::wire_token::WireTokenUsage;
use crate::session::file_tracking::FileHistorySnapshot;
use crate::session::types::base::SessionState;

/// Session ID type alias
pub use super::super::base::SessionId;

/// Unique message identifier (UUID)
pub type MessageId = String;

/// Branch identifier for sidechains
pub type BranchId = String;

/// Session metadata stored in metadata.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    /// Unique session ID
    pub id: SessionId,

    /// User-defined custom title
    #[serde(rename = "customTitle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,

    /// Session name (auto-generated or user-defined)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Preview of first user message
    #[serde(rename = "firstPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,

    /// Preview of last user message
    #[serde(rename = "lastPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_prompt: Option<String>,

    /// Auto-generated conversation summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// Working directory for this session
    #[serde(rename = "workingDirectory")]
    pub working_directory: PathBuf,

    /// Git branch (if in git repo)
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Model used for this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Sage version
    pub version: String,

    /// Total message count
    #[serde(rename = "messageCount")]
    pub message_count: usize,

    /// Session state
    pub state: SessionState,

    /// Token usage statistics
    #[serde(rename = "tokenUsage")]
    #[serde(default)]
    pub token_usage: WireTokenUsage,

    /// Whether this is a sidechain (branched session)
    #[serde(rename = "isSidechain")]
    #[serde(default)]
    pub is_sidechain: bool,

    /// Parent session ID (for sidechains)
    #[serde(rename = "parentSessionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<SessionId>,

    /// Root message ID where sidechain branched
    #[serde(rename = "sidechainRootMessageId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidechain_root_message_id: Option<MessageId>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl SessionHeader {
    /// Create a new session header
    pub fn new(id: impl Into<String>, working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            custom_title: None,
            name: None,
            first_prompt: None,
            last_prompt: None,
            summary: None,
            created_at: now,
            updated_at: now,
            working_directory,
            git_branch: None,
            model: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            message_count: 0,
            state: SessionState::Active,
            token_usage: WireTokenUsage::default(),
            is_sidechain: false,
            parent_session_id: None,
            sidechain_root_message_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set git branch
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Mark as sidechain
    pub fn as_sidechain(
        mut self,
        parent_session_id: impl Into<String>,
        root_message_id: impl Into<String>,
    ) -> Self {
        self.is_sidechain = true;
        self.parent_session_id = Some(parent_session_id.into());
        self.sidechain_root_message_id = Some(root_message_id.into());
        self
    }
}

/// Session aggregate view (in-memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session metadata
    pub header: SessionHeader,

    /// Messages (loaded from JSONL)
    #[serde(default)]
    pub messages: Vec<SessionMessage>,

    /// File history snapshots
    #[serde(default)]
    pub snapshots: Vec<FileHistorySnapshot>,
}

impl Session {
    /// Create a new session
    pub fn new(working_directory: PathBuf) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            header: SessionHeader::new(id, working_directory),
            messages: Vec::new(),
            snapshots: Vec::new(),
        }
    }

    /// Create with specific ID
    pub fn with_id(id: impl Into<String>, working_directory: PathBuf) -> Self {
        Self {
            header: SessionHeader::new(id, working_directory),
            messages: Vec::new(),
            snapshots: Vec::new(),
        }
    }
}
