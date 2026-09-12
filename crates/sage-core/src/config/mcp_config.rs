//! MCP (Model Context Protocol) configuration

use serde::ser::SerializeStruct;
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
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Whether MCP integration is enabled
    pub enabled: bool,
    /// MCP servers to connect to
    pub servers: HashMap<String, McpServerConfig>,
    /// Default timeout for MCP requests in seconds
    pub default_timeout_secs: u64,
    /// Whether default_timeout_secs was explicitly declared by a config source.
    pub default_timeout_secs_set: bool,
    /// Whether to auto-connect to servers on startup
    pub auto_connect: bool,
    /// Whether auto_connect was explicitly declared by a config source.
    pub auto_connect_set: bool,
    /// When true, description/schema trust baseline drift only warns and still
    /// registers the tool. Default false fails closed (skip drifted tools).
    pub warn_on_tool_trust_drift: bool,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            servers: HashMap::new(),
            default_timeout_secs: default_mcp_timeout(),
            default_timeout_secs_set: false,
            auto_connect: true,
            auto_connect_set: false,
            warn_on_tool_trust_drift: false,
        }
    }
}

impl Serialize for McpConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let include_timeout =
            self.default_timeout_secs_set || self.default_timeout_secs != default_mcp_timeout();
        let include_auto_connect = self.auto_connect_set || !self.auto_connect;
        let include_warn_on_drift = self.warn_on_tool_trust_drift;
        let len = 2
            + usize::from(include_timeout)
            + usize::from(include_auto_connect)
            + usize::from(include_warn_on_drift);
        let mut state = serializer.serialize_struct("McpConfig", len)?;
        state.serialize_field("enabled", &self.enabled)?;
        state.serialize_field("servers", &self.servers)?;
        if include_timeout {
            state.serialize_field("default_timeout_secs", &self.default_timeout_secs)?;
        }
        if include_auto_connect {
            state.serialize_field("auto_connect", &self.auto_connect)?;
        }
        if include_warn_on_drift {
            state.serialize_field("warn_on_tool_trust_drift", &self.warn_on_tool_trust_drift)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for McpConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct McpConfigWire {
            #[serde(default)]
            enabled: bool,
            #[serde(default)]
            servers: HashMap<String, McpServerConfig>,
            default_timeout_secs: Option<u64>,
            auto_connect: Option<bool>,
            #[serde(default)]
            warn_on_tool_trust_drift: bool,
        }

        let wire = McpConfigWire::deserialize(deserializer)?;
        Ok(Self {
            enabled: wire.enabled,
            servers: wire.servers,
            default_timeout_secs: wire
                .default_timeout_secs
                .unwrap_or_else(default_mcp_timeout),
            default_timeout_secs_set: wire.default_timeout_secs.is_some(),
            auto_connect: wire.auto_connect.unwrap_or_else(default_true),
            auto_connect_set: wire.auto_connect.is_some(),
            warn_on_tool_trust_drift: wire.warn_on_tool_trust_drift,
        })
    }
}

/// Authentication method declared by an MCP server config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpAuthKind {
    /// No authentication is required.
    None,
    /// A bearer token or equivalent shared secret is required.
    Bearer,
    /// OAuth authorization is required before tools may run.
    OAuth,
}

impl Default for McpAuthKind {
    fn default() -> Self {
        Self::None
    }
}

/// Authentication requirements for a single MCP server.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAuthConfig {
    /// Whether tools from this server require authentication before execution.
    #[serde(default)]
    pub required: bool,
    /// Authentication method used by the server.
    #[serde(default)]
    pub kind: McpAuthKind,
    /// Optional environment variable that must contain the token.
    pub token_env: Option<String>,
    /// Optional URL where the caller can complete authorization.
    pub authorization_url: Option<String>,
    /// Optional OAuth scopes or server-specific grants.
    #[serde(default)]
    pub scopes: Vec<String>,
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
    /// Authentication requirements for this server.
    #[serde(default)]
    pub auth: Option<McpAuthConfig>,
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

        if other.default_timeout_secs_set {
            self.default_timeout_secs = other.default_timeout_secs;
            self.default_timeout_secs_set = true;
        }

        if other.auto_connect_set || !other.auto_connect {
            self.auto_connect = other.auto_connect;
            self.auto_connect_set = true;
        }

        if other.warn_on_tool_trust_drift {
            self.warn_on_tool_trust_drift = true;
        }
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
            auth: None,
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
            auth: None,
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
            auth: None,
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

    /// Declare authentication requirements.
    pub fn with_auth(mut self, auth: McpAuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }
}

#[cfg(test)]
#[path = "mcp_config_tests.rs"]
mod mcp_config_tests;
