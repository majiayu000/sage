//! Implementation methods for cache data structures

use chrono::Utc;

use super::types::{RecentSession, SessionCacheData, ToolTrustSettings};

impl ToolTrustSettings {
    /// Check if a tool is allowed
    pub fn is_allowed(&self, tool_name: &str) -> Option<bool> {
        if self.always_allowed.contains(tool_name) {
            Some(true)
        } else if self.always_denied.contains(tool_name) {
            Some(false)
        } else {
            None // Requires confirmation or unknown
        }
    }

    /// Allow a tool
    pub fn allow(&mut self, tool_name: &str) {
        self.always_allowed.insert(tool_name.to_string());
        self.always_denied.remove(tool_name);
        self.require_confirmation.remove(tool_name);
        self.updated_at = Some(Utc::now());
    }

    /// Deny a tool
    pub fn deny(&mut self, tool_name: &str) {
        self.always_denied.insert(tool_name.to_string());
        self.always_allowed.remove(tool_name);
        self.require_confirmation.remove(tool_name);
        self.updated_at = Some(Utc::now());
    }

    /// Reset a tool to require confirmation
    pub fn reset(&mut self, tool_name: &str) {
        self.always_allowed.remove(tool_name);
        self.always_denied.remove(tool_name);
        self.require_confirmation.insert(tool_name.to_string());
        self.updated_at = Some(Utc::now());
    }
}

impl SessionCacheData {
    /// Current cache format version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            ..Default::default()
        }
    }

    /// Add a recent session
    pub fn add_recent_session(&mut self, session: RecentSession, max_sessions: usize) {
        // Remove existing entry for this ID
        self.recent_sessions.retain(|s| s.id != session.id);

        // Add to front
        self.recent_sessions.insert(0, session);

        // Trim to max
        if self.recent_sessions.len() > max_sessions {
            self.recent_sessions.truncate(max_sessions);
        }
    }

    /// Get a recent session by ID
    pub fn get_recent_session(&self, id: &str) -> Option<&RecentSession> {
        self.recent_sessions.iter().find(|s| s.id == id)
    }
}
