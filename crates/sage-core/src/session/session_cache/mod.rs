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

mod cache_ops;
mod data_ops;
mod manager;
mod persistence;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use manager::SessionCache;
pub use types::{
    McpServerCache, McpServerConfig, RecentSession, SessionCacheConfig, SessionCacheData,
    SessionCacheStats, ToolTrustSettings, UserPreferences,
};
