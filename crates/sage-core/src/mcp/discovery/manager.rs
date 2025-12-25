//! MCP Server Manager for discovery and lifecycle management

use super::scanner::discover_from_source;
use super::types::{DiscoverySource, ServerHealth, ServerStatus};
use super::utils::server_config_to_transport;
use crate::config::{McpConfig, McpServerConfig};
use crate::mcp::error::McpError;
use crate::mcp::registry::McpRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

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
            match discover_from_source(source).await {
                Ok((_, servers)) => {
                    for (name, config) in servers {
                        match self.connect_server(&name, config).await {
                            Ok(_) => {
                                info!("Connected to MCP server: {}", name);
                                connected_servers.push(name);
                            }
                            Err(e) => {
                                error!("Failed to connect to MCP server '{}': {}", name, e);
                                self.update_health(&name, ServerStatus::Failed(e.to_string()))
                                    .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to discover from source: {}", e);
                }
            }
        }

        Ok(connected_servers)
    }

    /// Discover servers from configuration
    pub async fn discover_from_config(&self, config: McpConfig) -> Result<Vec<String>, McpError> {
        if !config.enabled {
            debug!("MCP integration is disabled in config");
            return Ok(Vec::new());
        }

        let mut connected = Vec::new();

        for (name, server_config) in config.enabled_servers() {
            match self.connect_server(&name, server_config.clone()).await {
                Ok(_) => {
                    info!("Connected to MCP server: {}", name);
                    connected.push(name.clone());
                }
                Err(e) => {
                    error!("Failed to connect to MCP server '{}': {}", name, e);
                    self.update_health(&name, ServerStatus::Failed(e.to_string()))
                        .await;
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
