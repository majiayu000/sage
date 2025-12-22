//! Execution context for tool permission checking

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// Execution context for tool permission checking
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current working directory
    pub working_directory: PathBuf,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID
    pub agent_id: Option<String>,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Whether running in sandbox mode
    pub sandboxed: bool,
    /// Allowed paths for file operations
    pub allowed_paths: Vec<PathBuf>,
    /// Denied paths for file operations
    pub denied_paths: Vec<PathBuf>,
    /// Custom permissions
    pub custom_permissions: HashMap<String, bool>,
    /// Additional context data
    pub metadata: HashMap<String, Value>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_default(),
            session_id: None,
            agent_id: None,
            user_id: None,
            sandboxed: false,
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
            custom_permissions: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

impl ToolContext {
    /// Create a new tool context
    pub fn new(working_directory: PathBuf) -> Self {
        Self {
            working_directory,
            ..Default::default()
        }
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set agent ID
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Enable sandbox mode
    pub fn sandboxed(mut self) -> Self {
        self.sandboxed = true;
        self
    }

    /// Add allowed path
    pub fn allow_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.allowed_paths.push(path.into());
        self
    }

    /// Add denied path
    pub fn deny_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.denied_paths.push(path.into());
        self
    }

    /// Check if a path is allowed
    pub fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        // Check denied paths first
        for denied in &self.denied_paths {
            if path.starts_with(denied) {
                return false;
            }
        }

        // If no allowed paths specified, allow all (except denied)
        if self.allowed_paths.is_empty() {
            return true;
        }

        // Check allowed paths
        for allowed in &self.allowed_paths {
            if path.starts_with(allowed) {
                return true;
            }
        }

        false
    }

    /// Set a custom permission
    pub fn set_permission(mut self, key: impl Into<String>, allowed: bool) -> Self {
        self.custom_permissions.insert(key.into(), allowed);
        self
    }

    /// Check a custom permission
    pub fn has_permission(&self, key: &str) -> Option<bool> {
        self.custom_permissions.get(key).copied()
    }
}
