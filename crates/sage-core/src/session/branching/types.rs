//! Branch types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a branch
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(pub String);

impl BranchId {
    /// Create a new branch ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string()[..8].to_string())
    }

    /// Create from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for BranchId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A snapshot of conversation state at a branch point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSnapshot {
    /// Branch identifier
    pub id: BranchId,
    /// Branch name (user-provided or auto-generated)
    pub name: String,
    /// Description of the branch
    pub description: Option<String>,
    /// Parent branch (if any)
    pub parent_id: Option<BranchId>,
    /// When this branch was created
    pub created_at: DateTime<Utc>,
    /// Message index at branch point
    pub message_index: usize,
    /// Serialized messages up to this point
    pub messages: Vec<SerializedMessage>,
    /// Tool call history
    pub tool_history: Vec<SerializedToolCall>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for organization
    pub tags: Vec<String>,
}

impl BranchSnapshot {
    /// Create a new branch snapshot
    pub fn new(name: impl Into<String>, message_index: usize) -> Self {
        Self {
            id: BranchId::new(),
            name: name.into(),
            description: None,
            parent_id: None,
            created_at: Utc::now(),
            message_index,
            messages: Vec::new(),
            tool_history: Vec::new(),
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set parent branch
    pub fn with_parent(mut self, parent: BranchId) -> Self {
        self.parent_id = Some(parent);
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get age of the branch
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }

    /// Check if this is a root branch (no parent)
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Serialized message for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMessage {
    /// Role (user, assistant, system, tool)
    pub role: String,
    /// Content
    pub content: String,
    /// Optional name
    pub name: Option<String>,
    /// Tool call ID (for tool results)
    pub tool_call_id: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Serialized tool call for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedToolCall {
    /// Tool name
    pub tool_name: String,
    /// Arguments (JSON)
    pub arguments: serde_json::Value,
    /// Result
    pub result: Option<String>,
    /// Success status
    pub success: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Branch tree node for visualization
#[derive(Debug, Clone)]
pub struct BranchNode {
    /// The branch snapshot
    pub branch: BranchSnapshot,
    /// Child branches
    pub children: Vec<BranchId>,
    /// Depth in tree
    pub depth: usize,
}
