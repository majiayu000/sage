//! Cache operation methods for SessionCache

use crate::error::SageResult;
use chrono::Utc;

use super::manager::SessionCache;
use super::types::{
    McpServerCache, McpServerConfig, RecentSession, SessionCacheStats, ToolTrustSettings,
    UserPreferences,
};

impl SessionCache {
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
            let data = cache.get_or_insert_with(super::types::SessionCacheData::new);
            updater(&mut data.tool_trust);
        } else {
            let mut cache = self.global_cache.write().await;
            updater(&mut cache.tool_trust);
        }

        *self.dirty.write().await = true;
        Ok(())
    }

    /// Get MCP server configurations
    pub async fn get_mcp_servers(&self) -> McpServerCache {
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
    pub async fn set_mcp_server(&self, config: McpServerConfig) -> SageResult<()> {
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

    /// Get current session ID
    pub async fn get_current_session_id(&self) -> Option<String> {
        self.global_cache.read().await.current_session_id.clone()
    }

    /// Set current session ID
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
