//! Common types used throughout the Sage Agent system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for tasks, steps, and other entities
pub type Id = Uuid;

/// Token usage statistics for LLM calls
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LLMUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total number of tokens used
    pub total_tokens: u32,
    /// Cost in USD (if available)
    pub cost_usd: Option<f64>,
}

impl LLMUsage {
    /// Create a new LLMUsage instance
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cost_usd: None,
        }
    }

    /// Add usage from another LLMUsage instance
    pub fn add(&mut self, other: &LLMUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
        if let (Some(cost1), Some(cost2)) = (self.cost_usd, other.cost_usd) {
            self.cost_usd = Some(cost1 + cost2);
        }
    }
}

// AgentState is now defined in agent::state module

/// Task execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Task ID
    pub id: Id,
    /// Task description
    pub description: String,
    /// Working directory
    pub working_dir: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Additional metadata
    pub extra: HashMap<String, serde_json::Value>,
}

impl TaskMetadata {
    /// Create new task metadata
    pub fn new<S: Into<String>>(description: S, working_dir: S) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            working_dir: working_dir.into(),
            created_at: Utc::now(),
            completed_at: None,
            extra: HashMap::new(),
        }
    }

    /// Mark task as completed
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Get execution duration if completed
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed - self.created_at)
    }
}

/// File path with optional content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePath {
    /// The file path
    pub path: String,
    /// Optional file content (for small files)
    pub content: Option<String>,
    /// File size in bytes
    pub size: Option<u64>,
    /// Last modified timestamp
    pub modified: Option<DateTime<Utc>>,
}

impl FilePath {
    /// Create a new file path
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self {
            path: path.into(),
            content: None,
            size: None,
            modified: None,
        }
    }

    /// Create a file path with content
    pub fn with_content<S: Into<String>>(path: S, content: S) -> Self {
        let content_str = content.into();
        Self {
            path: path.into(),
            size: Some(content_str.len() as u64),
            content: Some(content_str),
            modified: Some(Utc::now()),
        }
    }
}

/// Git diff information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    /// The diff content
    pub content: String,
    /// Base commit hash
    pub base_commit: Option<String>,
    /// Target commit hash
    pub target_commit: Option<String>,
    /// Files changed
    pub files_changed: Vec<String>,
    /// Lines added
    pub lines_added: u32,
    /// Lines removed
    pub lines_removed: u32,
}

impl GitDiff {
    /// Create a new git diff
    pub fn new<S: Into<String>>(content: S) -> Self {
        Self {
            content: content.into(),
            base_commit: None,
            target_commit: None,
            files_changed: Vec::new(),
            lines_added: 0,
            lines_removed: 0,
        }
    }
}
