//! Type definitions for session caching

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Default cache file name
pub const CACHE_FILE_NAME: &str = "cache.json";
/// Global cache directory
pub const GLOBAL_CACHE_DIR: &str = ".sage";
/// Project cache directory
pub const PROJECT_CACHE_DIR: &str = ".sage";
/// Maximum recent sessions to track
pub const MAX_RECENT_SESSIONS: usize = 50;

/// Session cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCacheConfig {
    /// Whether to enable session caching
    pub enabled: bool,
    /// Path to global cache file
    pub global_cache_path: Option<PathBuf>,
    /// Whether to use project-local cache
    pub use_project_cache: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval_secs: u64,
    /// Maximum recent sessions to remember
    pub max_recent_sessions: usize,
}

impl Default for SessionCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            global_cache_path: None,
            use_project_cache: true,
            auto_save_interval_secs: 60,
            max_recent_sessions: MAX_RECENT_SESSIONS,
        }
    }
}

/// Cached tool trust settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolTrustSettings {
    /// Tools that are always allowed
    pub always_allowed: HashSet<String>,
    /// Tools that are always denied
    pub always_denied: HashSet<String>,
    /// Tools that require confirmation
    pub require_confirmation: HashSet<String>,
    /// Last updated timestamp
    pub updated_at: Option<DateTime<Utc>>,
}

/// MCP server configuration cache
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpServerCache {
    /// Server configurations by name
    pub servers: HashMap<String, McpServerConfig>,
    /// Last updated timestamp
    pub updated_at: Option<DateTime<Utc>>,
}

/// Cached MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name
    pub name: String,
    /// Command to start the server
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Whether this server is enabled
    pub enabled: bool,
    /// When this config was last used
    pub last_used: Option<DateTime<Utc>>,
}

/// Recent session entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentSession {
    /// Session ID
    pub id: String,
    /// Session name (if set)
    pub name: Option<String>,
    /// Working directory
    pub working_directory: String,
    /// Model used
    pub model: Option<String>,
    /// When the session was last active
    pub last_active: DateTime<Utc>,
    /// Number of messages in the session
    pub message_count: usize,
    /// Brief description or first message
    pub description: Option<String>,
}

/// User preferences cache
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Preferred model
    pub default_model: Option<String>,
    /// Preferred temperature
    pub default_temperature: Option<f32>,
    /// Auto-compact enabled
    pub auto_compact_enabled: bool,
    /// Theme preference
    pub theme: Option<String>,
    /// Editor preference
    pub editor: Option<String>,
    /// Custom settings
    pub custom: HashMap<String, serde_json::Value>,
}

/// The main session cache data structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCacheData {
    /// Version for cache format compatibility
    pub version: u32,
    /// Tool trust settings
    pub tool_trust: ToolTrustSettings,
    /// MCP server configurations
    pub mcp_servers: McpServerCache,
    /// Recent sessions
    pub recent_sessions: Vec<RecentSession>,
    /// User preferences
    pub preferences: UserPreferences,
    /// Current/last active session ID
    pub current_session_id: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When the cache was last saved
    pub last_saved: Option<DateTime<Utc>>,
}

/// Session cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCacheStats {
    pub recent_sessions_count: usize,
    pub mcp_servers_count: usize,
    pub allowed_tools_count: usize,
    pub denied_tools_count: usize,
    pub last_saved: Option<DateTime<Utc>>,
    pub has_project_cache: bool,
}
