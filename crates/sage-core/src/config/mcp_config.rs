//! MCP (Model Context Protocol) configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Helper function for default MCP timeout
fn default_mcp_timeout() -> u64 {
    300 // 5 minutes
}

/// Helper function for default true value
fn default_true() -> bool {
    true
}

/// MCP (Model Context Protocol) configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    /// Whether MCP integration is enabled
    #[serde(default)]
    pub enabled: bool,
    /// MCP servers to connect to
    #[serde(default)]
    pub servers: HashMap<String, McpServerConfig>,
    /// Default timeout for MCP requests in seconds
    #[serde(default = "default_mcp_timeout")]
    pub default_timeout_secs: u64,
    /// Whether to auto-connect to servers on startup
    #[serde(default = "default_true")]
    pub auto_connect: bool,
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Transport type: "stdio", "http", or "websocket"
    pub transport: String,
    /// Command to execute (for stdio transport)
    pub command: Option<String>,
    /// Command arguments (for stdio transport)
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables (for stdio transport)
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Base URL (for http/websocket transport)
    pub url: Option<String>,
    /// HTTP headers (for http transport)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Whether this server is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Request timeout in seconds (overrides default)
    pub timeout_secs: Option<u64>,
}

impl McpConfig {
    /// Merge with another MCP config (other takes precedence)
    pub fn merge(&mut self, other: McpConfig) {
        if other.enabled {
            self.enabled = true;
        }

        // Merge servers
        for (name, config) in other.servers {
            self.servers.insert(name, config);
        }

        if other.default_timeout_secs > 0 {
            self.default_timeout_secs = other.default_timeout_secs;
        }

        self.auto_connect = other.auto_connect;
    }

    /// Get enabled servers
    pub fn enabled_servers(&self) -> impl Iterator<Item = (&String, &McpServerConfig)> {
        self.servers.iter().filter(|(_, config)| config.enabled)
    }

    /// Get timeout for a specific server
    pub fn get_timeout(&self, server_name: &str) -> u64 {
        self.servers
            .get(server_name)
            .and_then(|s| s.timeout_secs)
            .unwrap_or(self.default_timeout_secs)
    }
}

impl McpServerConfig {
    /// Create a stdio transport config
    pub fn stdio(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            transport: "stdio".to_string(),
            command: Some(command.into()),
            args,
            env: HashMap::new(),
            url: None,
            headers: HashMap::new(),
            enabled: true,
            timeout_secs: None,
        }
    }

    /// Create an HTTP transport config
    pub fn http(url: impl Into<String>) -> Self {
        Self {
            transport: "http".to_string(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some(url.into()),
            headers: HashMap::new(),
            enabled: true,
            timeout_secs: None,
        }
    }

    /// Create a WebSocket transport config
    pub fn websocket(url: impl Into<String>) -> Self {
        Self {
            transport: "websocket".to_string(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some(url.into()),
            headers: HashMap::new(),
            enabled: true,
            timeout_secs: None,
        }
    }

    /// Add environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Add HTTP header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_default() {
        let config = McpConfig::default();
        assert!(!config.enabled);
        assert!(config.servers.is_empty());
        // Note: Default trait sets default_timeout_secs to 0
        // The default_mcp_timeout() is only used during deserialization
        assert_eq!(config.default_timeout_secs, 0);
        // default_true() is also only for deserialization, Default trait sets to false
        assert!(!config.auto_connect);
    }

    #[test]
    fn test_mcp_config_merge() {
        let mut config1 = McpConfig::default();
        let mut config2 = McpConfig::default();
        config2.enabled = true;
        config2.default_timeout_secs = 600;
        config2.auto_connect = false;
        config2
            .servers
            .insert("test".to_string(), McpServerConfig::stdio("test", vec![]));

        config1.merge(config2);
        assert!(config1.enabled);
        assert_eq!(config1.default_timeout_secs, 600);
        assert!(!config1.auto_connect);
        assert!(config1.servers.contains_key("test"));
    }

    #[test]
    fn test_mcp_config_enabled_servers() {
        let mut config = McpConfig::default();
        config.servers.insert(
            "enabled".to_string(),
            McpServerConfig::stdio("test", vec![]),
        );
        let mut disabled = McpServerConfig::stdio("test", vec![]);
        disabled.enabled = false;
        config.servers.insert("disabled".to_string(), disabled);

        let enabled: Vec<_> = config.enabled_servers().collect();
        assert_eq!(enabled.len(), 1);
        assert!(enabled[0].0 == "enabled");
    }

    #[test]
    fn test_mcp_config_get_timeout() {
        let mut config = McpConfig::default();
        config.default_timeout_secs = 300;

        // Server with custom timeout
        let mut server1 = McpServerConfig::stdio("test", vec![]);
        server1.timeout_secs = Some(120);
        config.servers.insert("custom".to_string(), server1);

        // Server without custom timeout
        let server2 = McpServerConfig::stdio("test", vec![]);
        config.servers.insert("default".to_string(), server2);

        assert_eq!(config.get_timeout("custom"), 120);
        assert_eq!(config.get_timeout("default"), 300);
        assert_eq!(config.get_timeout("nonexistent"), 300);
    }

    #[test]
    fn test_mcp_server_config_stdio() {
        let config = McpServerConfig::stdio("python", vec!["-m".to_string(), "test".to_string()]);
        assert_eq!(config.transport, "stdio");
        assert_eq!(config.command, Some("python".to_string()));
        assert_eq!(config.args, vec!["-m", "test"]);
        assert!(config.enabled);
    }

    #[test]
    fn test_mcp_server_config_http() {
        let config = McpServerConfig::http("http://localhost:8080");
        assert_eq!(config.transport, "http");
        assert_eq!(config.url, Some("http://localhost:8080".to_string()));
        assert!(config.enabled);
    }

    #[test]
    fn test_mcp_server_config_websocket() {
        let config = McpServerConfig::websocket("ws://localhost:9000");
        assert_eq!(config.transport, "websocket");
        assert_eq!(config.url, Some("ws://localhost:9000".to_string()));
        assert!(config.enabled);
    }

    #[test]
    fn test_mcp_server_config_with_env() {
        let config = McpServerConfig::stdio("test", vec![]).with_env("KEY", "value");
        assert_eq!(config.env.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_mcp_server_config_with_header() {
        let config =
            McpServerConfig::http("http://test").with_header("Authorization", "Bearer token");
        assert_eq!(
            config.headers.get("Authorization"),
            Some(&"Bearer token".to_string())
        );
    }

    #[test]
    fn test_mcp_server_config_with_timeout() {
        let config = McpServerConfig::stdio("test", vec![]).with_timeout(120);
        assert_eq!(config.timeout_secs, Some(120));
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_mcp_timeout(), 300);
        assert!(default_true());
    }
}
