//! Types for MCP server discovery and health tracking

use crate::config::McpConfig;
use std::path::PathBuf;

/// Discovery source for MCP servers
#[derive(Debug, Clone)]
pub enum DiscoverySource {
    /// From application configuration
    Config(McpConfig),
    /// From environment variable (JSON format)
    Environment(String),
    /// From file path
    File(PathBuf),
    /// From standard config locations
    Standard,
}

/// Server connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerStatus {
    /// Not yet connected
    Disconnected,
    /// Currently connecting
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection failed
    Failed(String),
}

/// Server health information
#[derive(Debug, Clone)]
pub struct ServerHealth {
    /// Server name
    pub name: String,
    /// Current status
    pub status: ServerStatus,
    /// Last successful ping time
    pub last_ping: Option<std::time::Instant>,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
}
