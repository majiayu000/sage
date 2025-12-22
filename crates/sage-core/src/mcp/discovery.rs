//! MCP server discovery and management
//!
//! Provides mechanisms for discovering and connecting to MCP servers from:
//! - Configuration files
//! - Environment variables
//! - Standard paths
//! - Dynamic registration

use super::error::McpError;
use super::registry::McpRegistry;
use super::transport::TransportConfig;
use crate::config::{McpConfig, McpServerConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

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

/// MCP Server Manager for discovery and lifecycle management
pub struct McpServerManager {
    /// The MCP registry for connected servers
    registry: Arc<McpRegistry>,
    /// Server health tracking
    health: RwLock<HashMap<String, ServerHealth>>,
    /// Original configuration for servers (for reconnection)
    server_configs: RwLock<HashMap<String, McpServerConfig>>,
    /// Default timeout
    #[allow(dead_code)]
    default_timeout: Duration,
}

impl McpServerManager {
    /// Create a new server manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(McpRegistry::new()),
            health: RwLock::new(HashMap::new()),
            server_configs: RwLock::new(HashMap::new()),
            default_timeout: Duration::from_secs(300),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            registry: Arc::new(McpRegistry::new()),
            health: RwLock::new(HashMap::new()),
            server_configs: RwLock::new(HashMap::new()),
            default_timeout: timeout,
        }
    }

    /// Get the underlying registry
    pub fn registry(&self) -> Arc<McpRegistry> {
        Arc::clone(&self.registry)
    }

    /// Discover and connect to servers from multiple sources
    pub async fn discover(&self, sources: Vec<DiscoverySource>) -> Result<Vec<String>, McpError> {
        let mut connected_servers = Vec::new();

        for source in sources {
            match self.discover_from_source(source).await {
                Ok(servers) => connected_servers.extend(servers),
                Err(e) => {
                    warn!("Failed to discover from source: {}", e);
                }
            }
        }

        Ok(connected_servers)
    }

    /// Discover from a single source
    async fn discover_from_source(&self, source: DiscoverySource) -> Result<Vec<String>, McpError> {
        match source {
            DiscoverySource::Config(config) => self.discover_from_config(config).await,
            DiscoverySource::Environment(var_name) => {
                self.discover_from_environment(&var_name).await
            }
            DiscoverySource::File(path) => self.discover_from_file(&path).await,
            DiscoverySource::Standard => self.discover_from_standard_paths().await,
        }
    }

    /// Discover servers from configuration
    pub async fn discover_from_config(&self, config: McpConfig) -> Result<Vec<String>, McpError> {
        if !config.enabled {
            debug!("MCP integration is disabled in config");
            return Ok(Vec::new());
        }

        let mut connected = Vec::new();

        for (name, server_config) in config.enabled_servers() {
            match self.connect_server(name, server_config.clone()).await {
                Ok(_) => {
                    info!("Connected to MCP server: {}", name);
                    connected.push(name.clone());
                }
                Err(e) => {
                    error!("Failed to connect to MCP server '{}': {}", name, e);
                    self.update_health(name, ServerStatus::Failed(e.to_string()))
                        .await;
                }
            }
        }

        Ok(connected)
    }

    /// Discover servers from environment variable
    async fn discover_from_environment(&self, var_name: &str) -> Result<Vec<String>, McpError> {
        let value = std::env::var(var_name).map_err(|_| {
            McpError::connection(format!("Environment variable {} not set", var_name))
        })?;

        let config: McpConfig = serde_json::from_str(&value)
            .map_err(|e| McpError::protocol(format!("Invalid JSON in {}: {}", var_name, e)))?;

        self.discover_from_config(config).await
    }

    /// Discover servers from a file
    async fn discover_from_file(&self, path: &PathBuf) -> Result<Vec<String>, McpError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| McpError::connection(format!("Failed to read file {:?}: {}", path, e)))?;

        let config: McpConfig = serde_json::from_str(&content)
            .map_err(|e| McpError::protocol(format!("Invalid JSON in {:?}: {}", path, e)))?;

        self.discover_from_config(config).await
    }

    /// Discover servers from standard paths
    async fn discover_from_standard_paths(&self) -> Result<Vec<String>, McpError> {
        let standard_paths = get_standard_mcp_paths();
        let mut connected = Vec::new();

        for path in standard_paths {
            if path.exists() {
                debug!("Checking standard MCP config path: {:?}", path);
                match self.discover_from_file(&path).await {
                    Ok(servers) => connected.extend(servers),
                    Err(e) => {
                        debug!("No valid MCP config at {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(connected)
    }

    /// Connect to a single server
    pub async fn connect_server(
        &self,
        name: &str,
        config: McpServerConfig,
    ) -> Result<(), McpError> {
        // Update status to connecting
        self.update_health(name, ServerStatus::Connecting).await;

        // Store config for potential reconnection
        {
            let mut configs = self.server_configs.write().await;
            configs.insert(name.to_string(), config.clone());
        }

        // Convert to transport config
        let transport_config = server_config_to_transport(&config)?;

        // Register with the registry
        match self.registry.register_server(name, transport_config).await {
            Ok(server_info) => {
                info!(
                    "MCP server '{}' connected: {} v{}",
                    name, server_info.name, server_info.version
                );
                self.update_health(name, ServerStatus::Connected).await;
                Ok(())
            }
            Err(e) => {
                self.update_health(name, ServerStatus::Failed(e.to_string()))
                    .await;
                Err(e)
            }
        }
    }

    /// Disconnect from a server
    pub async fn disconnect_server(&self, name: &str) -> Result<(), McpError> {
        self.registry.unregister_server(name).await?;
        self.update_health(name, ServerStatus::Disconnected).await;
        Ok(())
    }

    /// Reconnect to a server
    pub async fn reconnect_server(&self, name: &str) -> Result<(), McpError> {
        let config = {
            let configs = self.server_configs.read().await;
            configs.get(name).cloned().ok_or_else(|| {
                McpError::connection(format!("No config found for server: {}", name))
            })?
        };

        // Disconnect first (ignore errors)
        let _ = self.disconnect_server(name).await;

        // Reconnect
        self.connect_server(name, config).await
    }

    /// Get health status for all servers
    pub async fn health_status(&self) -> Vec<ServerHealth> {
        self.health.read().await.values().cloned().collect()
    }

    /// Get health status for a specific server
    pub async fn server_health(&self, name: &str) -> Option<ServerHealth> {
        self.health.read().await.get(name).cloned()
    }

    /// Check health of a server by pinging it
    pub async fn check_health(&self, name: &str) -> Result<bool, McpError> {
        if let Some(client) = self.registry.get_client(name) {
            match client.ping().await {
                Ok(_) => {
                    let mut health = self.health.write().await;
                    if let Some(h) = health.get_mut(name) {
                        h.last_ping = Some(std::time::Instant::now());
                        h.consecutive_failures = 0;
                        h.successful_requests += 1;
                    }
                    Ok(true)
                }
                Err(e) => {
                    let mut health = self.health.write().await;
                    if let Some(h) = health.get_mut(name) {
                        h.consecutive_failures += 1;
                        h.failed_requests += 1;
                    }
                    Err(e)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Check health of all servers
    pub async fn check_all_health(&self) -> HashMap<String, bool> {
        let server_names: Vec<String> = self.registry.server_names();
        let mut results = HashMap::new();

        for name in server_names {
            let healthy = self.check_health(&name).await.unwrap_or(false);
            results.insert(name, healthy);
        }

        results
    }

    /// Get list of connected server names
    pub fn connected_servers(&self) -> Vec<String> {
        self.registry.server_names()
    }

    /// Close all connections
    pub async fn close_all(&self) -> Result<(), McpError> {
        self.registry.close_all().await?;

        // Update all health statuses
        let mut health = self.health.write().await;
        for (_, h) in health.iter_mut() {
            h.status = ServerStatus::Disconnected;
        }

        Ok(())
    }

    /// Update health status for a server
    async fn update_health(&self, name: &str, status: ServerStatus) {
        let mut health = self.health.write().await;
        health
            .entry(name.to_string())
            .and_modify(|h| h.status = status.clone())
            .or_insert_with(|| ServerHealth {
                name: name.to_string(),
                status,
                last_ping: None,
                consecutive_failures: 0,
                successful_requests: 0,
                failed_requests: 0,
            });
    }
}

impl Default for McpServerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert McpServerConfig to TransportConfig
fn server_config_to_transport(config: &McpServerConfig) -> Result<TransportConfig, McpError> {
    match config.transport.as_str() {
        "stdio" => {
            let command = config.command.as_ref().ok_or_else(|| {
                McpError::invalid_request("Stdio transport requires command")
            })?;

            Ok(TransportConfig::Stdio {
                command: command.clone(),
                args: config.args.clone(),
                env: config.env.clone(),
            })
        }
        "http" => {
            let url = config
                .url
                .as_ref()
                .ok_or_else(|| McpError::invalid_request("HTTP transport requires url"))?;

            Ok(TransportConfig::Http {
                base_url: url.clone(),
                headers: config.headers.clone(),
            })
        }
        "websocket" => {
            let url = config.url.as_ref().ok_or_else(|| {
                McpError::invalid_request("WebSocket transport requires url")
            })?;

            Ok(TransportConfig::WebSocket { url: url.clone() })
        }
        other => Err(McpError::invalid_request(format!(
            "Unknown transport type: {}",
            other
        ))),
    }
}

/// Get standard MCP configuration paths
fn get_standard_mcp_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Current directory
    paths.push(PathBuf::from("mcp.json"));
    paths.push(PathBuf::from(".mcp.json"));

    // Home directory
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".config/sage/mcp.json"));
        paths.push(home.join(".sage/mcp.json"));
    }

    // XDG config directory
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("sage/mcp.json"));
    }

    paths
}

/// Builder for creating McpServerManager with custom settings
pub struct McpServerManagerBuilder {
    timeout: Duration,
    auto_reconnect: bool,
    health_check_interval: Option<Duration>,
}

impl McpServerManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            auto_reconnect: false,
            health_check_interval: None,
        }
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable auto-reconnect on failure
    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect = enabled;
        self
    }

    /// Set health check interval
    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = Some(interval);
        self
    }

    /// Build the manager
    pub fn build(self) -> McpServerManager {
        McpServerManager::with_timeout(self.timeout)
    }
}

impl Default for McpServerManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_to_transport_stdio() {
        let config = McpServerConfig::stdio("echo", vec!["hello".to_string()]);
        let transport = server_config_to_transport(&config).unwrap();

        assert!(matches!(transport, TransportConfig::Stdio { .. }));
    }

    #[test]
    fn test_server_config_to_transport_http() {
        let config = McpServerConfig::http("http://localhost:8080");
        let transport = server_config_to_transport(&config).unwrap();

        assert!(matches!(transport, TransportConfig::Http { .. }));
    }

    #[test]
    fn test_server_config_to_transport_websocket() {
        let config = McpServerConfig::websocket("ws://localhost:8080");
        let transport = server_config_to_transport(&config).unwrap();

        assert!(matches!(transport, TransportConfig::WebSocket { .. }));
    }

    #[test]
    fn test_server_config_to_transport_invalid() {
        let mut config = McpServerConfig::http("http://localhost:8080");
        config.transport = "invalid".to_string();

        let result = server_config_to_transport(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_standard_paths_not_empty() {
        let paths = get_standard_mcp_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_manager_creation() {
        let manager = McpServerManager::new();
        assert!(manager.connected_servers().is_empty());
    }
}
