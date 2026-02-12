//! Session context and thinking metadata types

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Session context embedded in each message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Current working directory
    pub cwd: PathBuf,

    /// Current git branch (if in git repo)
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Platform (macos, linux, windows)
    pub platform: String,

    /// User type (external, internal)
    #[serde(rename = "userType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
}

impl SessionContext {
    /// Create a new session context
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            git_branch: None,
            platform: std::env::consts::OS.to_string(),
            user_type: Some("external".to_string()),
        }
    }

    /// Create context with git branch
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Set user type
    pub fn with_user_type(mut self, user_type: impl Into<String>) -> Self {
        self.user_type = Some(user_type.into());
        self
    }

    /// Detect git branch from cwd
    pub fn detect_git_branch(&mut self) {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.cwd)
            .output()
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() {
                    self.git_branch = Some(branch);
                }
            }
        }
    }
}

/// Thinking level for extended thinking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    /// No extended thinking
    None,
    /// Low level thinking
    Low,
    /// Medium level thinking
    #[default]
    Medium,
    /// High level thinking (ultrathink)
    High,
}

impl fmt::Display for ThinkingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

/// Thinking metadata for extended thinking control
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThinkingMetadata {
    /// Thinking level
    pub level: ThinkingLevel,

    /// Whether extended thinking is disabled
    pub disabled: bool,

    /// Triggers that activated thinking
    #[serde(default)]
    pub triggers: Vec<String>,
}

impl ThinkingMetadata {
    /// Create default thinking metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific level
    pub fn with_level(level: ThinkingLevel) -> Self {
        Self {
            level,
            disabled: false,
            triggers: Vec::new(),
        }
    }

    /// Disable thinking
    pub fn disabled() -> Self {
        Self {
            level: ThinkingLevel::None,
            disabled: true,
            triggers: Vec::new(),
        }
    }

    /// Add a trigger
    pub fn with_trigger(mut self, trigger: impl Into<String>) -> Self {
        self.triggers.push(trigger.into());
        self
    }
}

/// Todo item and status (canonical definition in `crate::types::todo`)
pub use crate::types::{TodoItem, TodoStatus};
