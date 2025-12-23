//! Task types and state definitions

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task state enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskState {
    #[serde(rename = "NOT_STARTED")]
    NotStarted,
    #[serde(rename = "IN_PROGRESS")]
    InProgress,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "COMPLETE")]
    Complete,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::NotStarted => write!(f, "[ ]"),
            TaskState::InProgress => write!(f, "[/]"),
            TaskState::Cancelled => write!(f, "[-]"),
            TaskState::Complete => write!(f, "[x]"),
        }
    }
}

/// Individual task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub state: TaskState,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Task {
    pub fn new(name: String, description: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            state: TaskState::NotStarted,
            parent_id: None,
            children: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}
