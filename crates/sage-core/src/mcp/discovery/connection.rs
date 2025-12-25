//! Server connection management

use super::health::HealthTracker;
use super::types::ServerStatus;
use super::utils::server_config_to_transport;
use crate::config::McpServerConfig;
use crate::mcp::error::McpError;
use crate::mcp::registry::McpRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Connection manager for server lifecycle
pub(super) struct ConnectionManager {
    server_configs: RwLock<HashMap<String, McpServerConfig>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            server_configs: RwLock::new(HashMap::new()),
        }
    }

    /// Connect to a single server
    pub async fn connect_server(
        &self,
        name: &str,
        config: McpServerConfig,
        registry: &Arc<McpRegistry>,
        health_tracker: &HealthTracker,
    ) -> Result<(), McpError> {
        // Update status to connecting
        health_tracker
            .update_health(name, ServerStatus::Connecting)
            .await;

        // Store config for potential reconnection
        {
            let mut configs = self.server_configs.write().await;
            configs.insert(name.to_string(), config.clone());
        }

        // Convert to transport config
        let transport_config = server_config_to_transport(&config)?;

        // Register with the registry
        match registry.register_server(name, transport_config).await {
            Ok(server_info) => {
                info!(
                    "MCP server '{}' connected: {} v{}",
                    name, server_info.name, server_info.version
                );
                health_tracker
                    .update_health(name, ServerStatus::Connected)
                    .await;
                Ok(())
            }
            Err(e) => {
                health_tracker
                    .update_health(name, ServerStatus::Failed(e.to_string()))
                    .await;
                Err(e)
            }
        }
    }

    /// Disconnect from a server
    pub async fn disconnect_server(
        &self,
        name: &str,
        registry: &Arc<McpRegistry>,
        health_tracker: &HealthTracker,
    ) -> Result<(), McpError> {
        registry.unregister_server(name).await?;
        health_tracker
            .update_health(name, ServerStatus::Disconnected)
            .await;
        Ok(())
    }

    /// Reconnect to a server
    pub async fn reconnect_server(
        &self,
        name: &str,
        registry: &Arc<McpRegistry>,
        health_tracker: &HealthTracker,
    ) -> Result<(), McpError> {
        let config = {
            let configs = self.server_configs.read().await;
            configs.get(name).cloned().ok_or_else(|| {
                McpError::connection(format!("No config found for server: {}", name))
            })?
        };

        // Disconnect first (ignore errors)
        let _ = self.disconnect_server(name, registry, health_tracker).await;

        // Reconnect
        self.connect_server(name, config, registry, health_tracker)
            .await
    }
}
