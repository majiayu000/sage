//! MCP Tool Registry
//!
//! Manages MCP server connections and provides tools to the Sage agent.

use sage_core::config::{McpConfig, McpServerConfig};
use sage_core::mcp::{
    HttpTransport, HttpTransportConfig, McpClient, McpToolAdapter, StdioTransport,
};
use sage_core::tools::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Registry for MCP tools
pub struct McpToolRegistry {
    /// Connected clients by server name
    clients: RwLock<HashMap<String, Arc<McpClient>>>,
    /// All available tools (server_name::tool_name -> adapter)
    tools: RwLock<Vec<McpToolAdapter>>,
    /// Server status tracking
    server_status: RwLock<HashMap<String, ServerConnectionStatus>>,
}

/// Connection status for an MCP server
#[derive(Debug, Clone)]
pub struct ServerConnectionStatus {
    /// Server name
    pub name: String,
    /// Whether connected
    pub connected: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of tools available
    pub tool_count: usize,
}

impl McpToolRegistry {
    /// Create a new MCP tool registry
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            tools: RwLock::new(Vec::new()),
            server_status: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize from MCP configuration
    pub async fn from_config(config: &McpConfig) -> Result<Self, String> {
        let registry = Self::new();

        if !config.enabled {
            debug!("MCP integration is disabled in config");
            return Ok(registry);
        }

        for (name, server_config) in &config.servers {
            if !server_config.enabled {
                debug!("Skipping disabled MCP server: {}", name);
                continue;
            }

            match registry.connect_server(name, server_config).await {
                Ok(_) => info!("Connected to MCP server: {}", name),
                Err(e) => {
                    warn!("Failed to connect to MCP server {}: {}", name, e);
                    // Record the failure but continue with other servers
                    let mut status = registry.server_status.write().await;
                    let name_owned = name.clone();
                    status.insert(
                        name_owned.clone(),
                        ServerConnectionStatus {
                            name: name_owned,
                            connected: false,
                            error: Some(e),
                            tool_count: 0,
                        },
                    );
                }
            }
        }

        Ok(registry)
    }

    /// Connect to an MCP server
    pub async fn connect_server(
        &self,
        name: &str,
        config: &McpServerConfig,
    ) -> Result<usize, String> {
        let client = match config.transport.as_str() {
            "stdio" => self.connect_stdio(config).await?,
            "http" => self.connect_http(config).await?,
            _ => return Err(format!("Unsupported transport: {}", config.transport)),
        };

        let client = Arc::new(client);

        // Get tools from the server
        let tools_list = client
            .list_tools()
            .await
            .map_err(|e| format!("Failed to list MCP tools: {}", e))?;
        let adapters: Vec<McpToolAdapter> = tools_list
            .into_iter()
            .map(|tool| McpToolAdapter::new(tool, Arc::clone(&client), name.to_string()))
            .collect();
        let tool_count = adapters.len();

        // Store the client
        let mut clients = self.clients.write().await;
        clients.insert(name.to_string(), Arc::clone(&client));

        // Store the tools
        let mut tools = self.tools.write().await;
        tools.extend(adapters);

        // Update status
        let mut status = self.server_status.write().await;
        status.insert(
            name.to_string(),
            ServerConnectionStatus {
                name: name.to_string(),
                connected: true,
                error: None,
                tool_count,
            },
        );

        Ok(tool_count)
    }

    /// Connect using stdio transport
    async fn connect_stdio(&self, config: &McpServerConfig) -> Result<McpClient, String> {
        let command = config
            .command
            .as_ref()
            .ok_or_else(|| "stdio transport requires 'command' field".to_string())?;

        let transport = StdioTransport::spawn(command, &config.args)
            .await
            .map_err(|e| format!("Failed to spawn stdio transport: {}", e))?;

        let client = McpClient::new(Box::new(transport));

        client
            .initialize()
            .await
            .map_err(|e| format!("Failed to initialize MCP client: {}", e))?;

        Ok(client)
    }

    /// Connect using HTTP transport
    async fn connect_http(&self, config: &McpServerConfig) -> Result<McpClient, String> {
        let url = config
            .url
            .as_ref()
            .ok_or_else(|| "http transport requires 'url' field".to_string())?;

        let mut http_config =
            HttpTransportConfig::new(url).with_timeout(config.timeout_secs.unwrap_or(300));

        // Add configured headers
        for (key, value) in &config.headers {
            http_config = http_config.with_header(key, value);
        }

        let transport = HttpTransport::new(http_config)
            .map_err(|e| format!("Failed to create HTTP transport: {}", e))?;

        let client = McpClient::new(Box::new(transport));

        client
            .initialize()
            .await
            .map_err(|e| format!("Failed to initialize MCP client: {}", e))?;

        Ok(client)
    }

    /// Disconnect from an MCP server
    pub async fn disconnect_server(&self, name: &str) -> Result<(), String> {
        let mut clients = self.clients.write().await;

        if clients.remove(name).is_none() {
            return Err(format!("Server '{}' not connected", name));
        }

        // Remove tools from this server
        let mut tools = self.tools.write().await;
        tools.retain(|t| t.server_name() != name);

        // Update status
        let mut status = self.server_status.write().await;
        if let Some(s) = status.get_mut(name) {
            s.connected = false;
            s.tool_count = 0;
        }

        Ok(())
    }

    /// Get all available tools
    pub async fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools
            .iter()
            .cloned()
            .map(|t| Arc::new(t) as Arc<dyn Tool>)
            .collect()
    }

    /// Get a specific tool by name
    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools
            .iter()
            .find(|t| t.name() == name)
            .cloned()
            .map(|t| Arc::new(t) as Arc<dyn Tool>)
    }

    /// Get tools from a specific server
    pub async fn tools_from_server(&self, server_name: &str) -> Vec<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools
            .iter()
            .filter(|t| t.server_name() == server_name)
            .cloned()
            .map(|t| Arc::new(t) as Arc<dyn Tool>)
            .collect()
    }

    /// Get tool names organized by server
    pub async fn tool_names_by_server(&self) -> HashMap<String, Vec<String>> {
        let tools = self.tools.read().await;
        let mut result: HashMap<String, Vec<String>> = HashMap::new();

        for tool in tools.iter() {
            result
                .entry(tool.server_name().to_string())
                .or_default()
                .push(tool.name().to_string());
        }

        result
    }

    /// Get connection status for all servers
    pub async fn server_statuses(&self) -> Vec<ServerConnectionStatus> {
        let status = self.server_status.read().await;
        status.values().cloned().collect()
    }

    /// Check if a server is connected
    pub async fn is_connected(&self, name: &str) -> bool {
        let status = self.server_status.read().await;
        status.get(name).map(|s| s.connected).unwrap_or(false)
    }

    /// Get total number of available tools
    pub async fn tool_count(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Get number of connected servers
    pub async fn server_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared MCP tool registry
pub type SharedMcpToolRegistry = Arc<McpToolRegistry>;

/// Create a shared MCP tool registry from config
pub async fn create_mcp_registry(config: &McpConfig) -> Result<SharedMcpToolRegistry, String> {
    let registry = McpToolRegistry::from_config(config).await?;
    Ok(Arc::new(registry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_registry() {
        let registry = McpToolRegistry::new();

        assert_eq!(registry.tool_count().await, 0);
        assert_eq!(registry.server_count().await, 0);
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let config = McpConfig {
            enabled: false,
            servers: HashMap::new(),
            default_timeout_secs: 300,
            auto_connect: true,
        };

        let registry = McpToolRegistry::from_config(&config).await.unwrap();
        assert_eq!(registry.tool_count().await, 0);
    }
}
