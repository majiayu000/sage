//! Session caching for persistent state across sessions
//!
//! This module provides session-level caching similar to Claude Code's `~/.claude.json`.
//! It stores runtime state, preferences, and configurations that persist between sessions.
//!
//! ## Cached Data
//!
//! - Active session state and preferences
//! - MCP server configurations
//! - Tool trust settings (allowed/denied tools)
//! - Recent session history
//! - User preferences
//!
//! ## Storage
//!
//! Data is stored in:
//! - Global: `~/.sage/cache.json`
//! - Project-specific: `.sage/cache.json`

use crate::error::{SageError, SageResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Default cache file name
const CACHE_FILE_NAME: &str = "cache.json";
/// Global cache directory
const GLOBAL_CACHE_DIR: &str = ".sage";
/// Project cache directory
const PROJECT_CACHE_DIR: &str = ".sage";
/// Maximum recent sessions to track
const MAX_RECENT_SESSIONS: usize = 50;

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

/// MCP server configuration cache
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpServerCache {
    /// Server configurations by name
    pub servers: HashMap<String, McpServerConfig>,
    /// Last updated timestamp
    pub updated_at: Option<DateTime<Utc>>,
}

/// Deprecated: Use `McpServerCache` instead
#[deprecated(since = "0.2.0", note = "Use `McpServerCache` instead")]
pub type MCPServerCache = McpServerCache;

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

/// Deprecated: Use `McpServerConfig` instead
#[deprecated(since = "0.2.0", note = "Use `McpServerConfig` instead")]
pub type MCPServerConfig = McpServerConfig;

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
    pub mcp_servers: MCPServerCache,
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

impl SessionCacheData {
    /// Current cache format version
    const CURRENT_VERSION: u32 = 1;

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

/// Session cache manager
pub struct SessionCache {
    /// Configuration
    config: SessionCacheConfig,
    /// Global cache data
    global_cache: Arc<RwLock<SessionCacheData>>,
    /// Project-specific cache data
    project_cache: Arc<RwLock<Option<SessionCacheData>>>,
    /// Global cache file path
    global_path: PathBuf,
    /// Project cache file path (if any)
    project_path: Option<PathBuf>,
    /// Whether cache has unsaved changes
    dirty: Arc<RwLock<bool>>,
}

impl SessionCache {
    /// Create a new session cache
    pub async fn new(config: SessionCacheConfig) -> SageResult<Self> {
        let global_path = config.global_cache_path.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(GLOBAL_CACHE_DIR)
                .join(CACHE_FILE_NAME)
        });

        let cache = Self {
            config,
            global_cache: Arc::new(RwLock::new(SessionCacheData::new())),
            project_cache: Arc::new(RwLock::new(None)),
            global_path,
            project_path: None,
            dirty: Arc::new(RwLock::new(false)),
        };

        // Load existing cache
        cache.load_global().await?;

        Ok(cache)
    }

    /// Initialize with project directory
    pub async fn with_project_dir(mut self, project_dir: &Path) -> SageResult<Self> {
        if self.config.use_project_cache {
            self.project_path = Some(project_dir.join(PROJECT_CACHE_DIR).join(CACHE_FILE_NAME));
            self.load_project().await?;
        }
        Ok(self)
    }

    /// Load global cache from disk
    async fn load_global(&self) -> SageResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.global_path.exists() {
            match fs::read_to_string(&self.global_path).await {
                Ok(content) => match serde_json::from_str::<SessionCacheData>(&content) {
                    Ok(data) => {
                        *self.global_cache.write().await = data;
                        tracing::debug!("Loaded global session cache from {:?}", self.global_path);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse global cache: {}, using defaults", e);
                    }
                },
                Err(e) => {
                    tracing::debug!("No global cache file found: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Load project cache from disk
    async fn load_project(&self) -> SageResult<()> {
        if !self.config.enabled || !self.config.use_project_cache {
            return Ok(());
        }

        if let Some(path) = &self.project_path {
            if path.exists() {
                match fs::read_to_string(path).await {
                    Ok(content) => match serde_json::from_str::<SessionCacheData>(&content) {
                        Ok(data) => {
                            *self.project_cache.write().await = Some(data);
                            tracing::debug!("Loaded project session cache from {:?}", path);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse project cache: {}", e);
                        }
                    },
                    Err(e) => {
                        tracing::debug!("No project cache file: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Save cache to disk
    pub async fn save(&self) -> SageResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Save global cache
        self.save_to_path(&self.global_path, &*self.global_cache.read().await)
            .await?;

        // Save project cache if exists
        if let Some(path) = &self.project_path {
            if let Some(data) = &*self.project_cache.read().await {
                self.save_to_path(path, data).await?;
            }
        }

        *self.dirty.write().await = false;
        Ok(())
    }

    /// Save cache data to a specific path
    async fn save_to_path(&self, path: &Path, data: &SessionCacheData) -> SageResult<()> {
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| SageError::io(format!("Failed to create cache directory: {}", e)))?;
        }

        // Update last saved timestamp
        let mut data = data.clone();
        data.last_saved = Some(Utc::now());

        let content = serde_json::to_string_pretty(&data)?;
        fs::write(path, content)
            .await
            .map_err(|e| SageError::io(format!("Failed to write cache: {}", e)))?;

        tracing::debug!("Saved session cache to {:?}", path);
        Ok(())
    }

    /// Get tool trust settings (merges global and project)
    pub async fn get_tool_trust(&self) -> ToolTrustSettings {
        let global = self.global_cache.read().await;
        let mut trust = global.tool_trust.clone();

        // Project settings override global
        if let Some(project) = &*self.project_cache.read().await {
            trust
                .always_allowed
                .extend(project.tool_trust.always_allowed.clone());
            trust
                .always_denied
                .extend(project.tool_trust.always_denied.clone());
        }

        trust
    }

    /// Update tool trust (saves to project cache if available, otherwise global)
    pub async fn update_tool_trust<F>(&self, updater: F) -> SageResult<()>
    where
        F: FnOnce(&mut ToolTrustSettings),
    {
        if self.project_path.is_some() {
            let mut cache = self.project_cache.write().await;
            let data = cache.get_or_insert_with(SessionCacheData::new);
            updater(&mut data.tool_trust);
        } else {
            let mut cache = self.global_cache.write().await;
            updater(&mut cache.tool_trust);
        }

        *self.dirty.write().await = true;
        Ok(())
    }

    /// Get MCP server configurations
    pub async fn get_mcp_servers(&self) -> MCPServerCache {
        let global = self.global_cache.read().await;
        let mut servers = global.mcp_servers.clone();

        // Project servers extend global
        if let Some(project) = &*self.project_cache.read().await {
            for (name, config) in &project.mcp_servers.servers {
                servers.servers.insert(name.clone(), config.clone());
            }
        }

        servers
    }

    /// Add or update MCP server configuration
    pub async fn set_mcp_server(&self, config: MCPServerConfig) -> SageResult<()> {
        let mut cache = self.global_cache.write().await;
        cache
            .mcp_servers
            .servers
            .insert(config.name.clone(), config);
        cache.mcp_servers.updated_at = Some(Utc::now());

        *self.dirty.write().await = true;
        Ok(())
    }

    /// Get recent sessions
    pub async fn get_recent_sessions(&self) -> Vec<RecentSession> {
        self.global_cache.read().await.recent_sessions.clone()
    }

    /// Add a recent session
    pub async fn add_recent_session(&self, session: RecentSession) -> SageResult<()> {
        let mut cache = self.global_cache.write().await;
        cache.add_recent_session(session, self.config.max_recent_sessions);

        *self.dirty.write().await = true;
        Ok(())
    }

    /// Get user preferences
    pub async fn get_preferences(&self) -> UserPreferences {
        self.global_cache.read().await.preferences.clone()
    }

    /// Update user preferences
    pub async fn update_preferences<F>(&self, updater: F) -> SageResult<()>
    where
        F: FnOnce(&mut UserPreferences),
    {
        let mut cache = self.global_cache.write().await;
        updater(&mut cache.preferences);

        *self.dirty.write().await = true;
        Ok(())
    }

    /// Get/set current session ID
    pub async fn get_current_session_id(&self) -> Option<String> {
        self.global_cache.read().await.current_session_id.clone()
    }

    pub async fn set_current_session_id(&self, id: Option<String>) -> SageResult<()> {
        self.global_cache.write().await.current_session_id = id;
        *self.dirty.write().await = true;
        Ok(())
    }

    /// Check if cache has unsaved changes
    pub async fn is_dirty(&self) -> bool {
        *self.dirty.read().await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> SessionCacheStats {
        let global = self.global_cache.read().await;
        SessionCacheStats {
            recent_sessions_count: global.recent_sessions.len(),
            mcp_servers_count: global.mcp_servers.servers.len(),
            allowed_tools_count: global.tool_trust.always_allowed.len(),
            denied_tools_count: global.tool_trust.always_denied.len(),
            last_saved: global.last_saved,
            has_project_cache: self.project_cache.blocking_read().is_some(),
        }
    }
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

impl Default for SessionCache {
    fn default() -> Self {
        let config = SessionCacheConfig::default();
        let global_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(GLOBAL_CACHE_DIR)
            .join(CACHE_FILE_NAME);

        Self {
            config,
            global_cache: Arc::new(RwLock::new(SessionCacheData::new())),
            project_cache: Arc::new(RwLock::new(None)),
            global_path,
            project_path: None,
            dirty: Arc::new(RwLock::new(false)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_cache_creation() {
        let config = SessionCacheConfig {
            enabled: true,
            global_cache_path: Some(PathBuf::from("/tmp/test_cache.json")),
            ..Default::default()
        };

        let cache = SessionCache::new(config).await.unwrap();
        assert!(!cache.is_dirty().await);
    }

    #[tokio::test]
    async fn test_tool_trust_settings() {
        let config = SessionCacheConfig {
            enabled: true,
            global_cache_path: Some(PathBuf::from("/tmp/test_cache2.json")),
            ..Default::default()
        };

        let cache = SessionCache::new(config).await.unwrap();

        // Initially no trust settings
        let trust = cache.get_tool_trust().await;
        assert!(trust.always_allowed.is_empty());

        // Allow a tool
        cache.update_tool_trust(|t| t.allow("bash")).await.unwrap();

        let trust = cache.get_tool_trust().await;
        assert!(trust.always_allowed.contains("bash"));
        assert!(cache.is_dirty().await);
    }

    #[tokio::test]
    async fn test_recent_sessions() {
        let config = SessionCacheConfig {
            enabled: true,
            global_cache_path: Some(PathBuf::from("/tmp/test_cache3.json")),
            max_recent_sessions: 5,
            ..Default::default()
        };

        let cache = SessionCache::new(config).await.unwrap();

        // Add sessions
        for i in 0..7 {
            cache
                .add_recent_session(RecentSession {
                    id: format!("session-{}", i),
                    name: Some(format!("Session {}", i)),
                    working_directory: "/tmp".to_string(),
                    model: Some("claude-3.5-sonnet".to_string()),
                    last_active: Utc::now(),
                    message_count: i * 10,
                    description: None,
                })
                .await
                .unwrap();
        }

        let sessions = cache.get_recent_sessions().await;
        assert_eq!(sessions.len(), 5); // Max is 5
        assert_eq!(sessions[0].id, "session-6"); // Most recent first
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        // Create and populate cache
        {
            let config = SessionCacheConfig {
                enabled: true,
                global_cache_path: Some(cache_path.clone()),
                ..Default::default()
            };

            let cache = SessionCache::new(config).await.unwrap();

            cache
                .update_tool_trust(|t| {
                    t.allow("read");
                    t.deny("rm");
                })
                .await
                .unwrap();

            cache
                .update_preferences(|p| {
                    p.default_model = Some("claude-3.5-sonnet".to_string());
                })
                .await
                .unwrap();

            cache.save().await.unwrap();
        }

        // Load and verify
        {
            let config = SessionCacheConfig {
                enabled: true,
                global_cache_path: Some(cache_path),
                ..Default::default()
            };

            let cache = SessionCache::new(config).await.unwrap();

            let trust = cache.get_tool_trust().await;
            assert!(trust.always_allowed.contains("read"));
            assert!(trust.always_denied.contains("rm"));

            let prefs = cache.get_preferences().await;
            assert_eq!(prefs.default_model, Some("claude-3.5-sonnet".to_string()));
        }
    }
}
