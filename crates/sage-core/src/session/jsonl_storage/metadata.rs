//! Session metadata type and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::super::types::SessionId;

/// Session metadata stored in metadata.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session ID
    pub id: SessionId,

    /// Session name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// Working directory
    #[serde(rename = "workingDirectory")]
    pub working_directory: PathBuf,

    /// Git branch at session start
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Sage Agent version
    pub version: String,

    /// Total message count
    #[serde(rename = "messageCount")]
    pub message_count: usize,

    /// Session state (active, completed, failed)
    pub state: String,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionMetadata {
    /// Create new session metadata
    pub fn new(id: impl Into<String>, working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: None,
            created_at: now,
            updated_at: now,
            working_directory,
            git_branch: None,
            model: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            message_count: 0,
            state: "active".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Set session name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
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

    /// Update message count
    pub fn update_message_count(&mut self, count: usize) {
        self.message_count = count;
        self.updated_at = Utc::now();
    }

    /// Set state
    pub fn set_state(&mut self, state: impl Into<String>) {
        self.state = state.into();
        self.updated_at = Utc::now();
    }
}
