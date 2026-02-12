//! Todo item types shared across session, agent, and sage-tools modules

use serde::{Deserialize, Serialize};
use std::fmt;

/// Todo item for task tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Task content (imperative form)
    pub content: String,

    /// Task status
    pub status: TodoStatus,

    /// Active form (present continuous)
    #[serde(rename = "activeForm")]
    pub active_form: String,
}

/// Todo status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// Task not yet started
    Pending,
    /// Task in progress
    InProgress,
    /// Task completed
    Completed,
}

impl fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
        }
    }
}
