//! MCP Server Manager for discovery and lifecycle management

use super::connection::ConnectionManager;
use super::health::HealthTracker;
use super::scanner::discover_from_source;
use super::types::{DiscoverySource, ServerHealth, ServerStatus};
use crate::config::{McpConfig, McpServerConfig};
use crate::mcp::error::McpError;
use crate::mcp::registry::McpRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// MCP Server Manager for discovery and lifecycle management
pub struct McpServerManager {
    /// The MCP registry for connected servers
    registry: Arc<McpRegistry>,
    /// Server health tracking
    health_tracker: HealthTracker,
    /// Connection management
    connection_manager: ConnectionManager,
    /// Default timeout
    #[allow(dead_code)]
    default_timeout: Duration,
}

impl McpServerManager {
    /// Create a new server manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(McpRegistry::new()),
            health_tracker: HealthTracker::new(),
            connection_manager: ConnectionManager::new(),
            default_timeout: Duration::from_secs(300),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            registry: Arc::new(McpRegistry::new()),
            health_tracker: HealthTracker::new(),
            connection_manager: ConnectionManager::new(),
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
                        match self
                            .connection_manager
                            .connect_server(&name, config, &self.registry, &self.health_tracker)
                            .await
                        {
                            Ok(_) => {
                                info!("Connected to MCP server: {}", name);
                                connected_servers.push(name);
                            }
                            Err(e) => {
                                error!("Failed to connect to MCP server '{}': {}", name, e);
                                self.health_tracker
                                    .update_health(&name, ServerStatus::Failed(e.to_string()))
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
            match self
                .connection_manager
                .connect_server(name, server_config.clone(), &self.registry, &self.health_tracker)
                .await
            {
                Ok(_) => {
                    info!("Connected to MCP server: {}", name);
                    connected.push(name.clone());
                }
                Err(e) => {
                    error!("Failed to connect to MCP server '{}': {}", name, e);
                    self.health_tracker
                        .update_health(name, ServerStatus::Failed(e.to_string()))
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
        self.connection_manager
            .connect_server(name, config, &self.registry, &self.health_tracker)
            .await
    }

    /// Disconnect from a server
    pub async fn disconnect_server(&self, name: &str) -> Result<(), McpError> {
        self.connection_manager
            .disconnect_server(name, &self.registry, &self.health_tracker)
            .await
    }

    /// Reconnect to a server
    pub async fn reconnect_server(&self, name: &str) -> Result<(), McpError> {
        self.connection_manager
            .reconnect_server(name, &self.registry, &self.health_tracker)
            .await
    }

    /// Get health status for all servers
    pub async fn health_status(&self) -> Vec<ServerHealth> {
        self.health_tracker.health_status().await
    }

    /// Get health status for a specific server
    pub async fn server_health(&self, name: &str) -> Option<ServerHealth> {
        self.health_tracker.server_health(name).await
    }

    /// Check health of a server by pinging it
    pub async fn check_health(&self, name: &str) -> Result<bool, McpError> {
        self.health_tracker.check_health(name, &self.registry).await
    }

    /// Check health of all servers
    pub async fn check_all_health(&self) -> HashMap<String, bool> {
        let server_names = self.registry.server_names();
        self.health_tracker
            .check_all_health(&server_names, &self.registry)
            .await
    }

    /// Get list of connected server names
    pub fn connected_servers(&self) -> Vec<String> {
        self.registry.server_names()
    }

    /// Close all connections
    pub async fn close_all(&self) -> Result<(), McpError> {
        self.registry.close_all().await?;
        self.health_tracker.mark_all_disconnected().await;
        Ok(())
    }
}

impl Default for McpServerManager {
    fn default() -> Self {
        Self::new()
    }
}
