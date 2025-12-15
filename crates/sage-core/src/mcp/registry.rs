//! MCP registry for managing multiple MCP servers
//!
//! Provides centralized management of MCP servers and their tools.

use super::client::McpClient;
use super::error::McpError;
use super::transport::{HttpTransport, HttpTransportConfig, StdioTransport, TransportConfig};
use super::types::{McpPrompt, McpResource, McpServerInfo, McpTool};
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;

/// Registry for managing MCP servers and their capabilities
pub struct McpRegistry {
    /// Connected MCP clients by name
    clients: DashMap<String, Arc<McpClient>>,
    /// Tool to client mapping
    tool_mapping: DashMap<String, String>,
    /// Resource to client mapping
    resource_mapping: DashMap<String, String>,
    /// Prompt to client mapping
    prompt_mapping: DashMap<String, String>,
}

impl McpRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            clients: DashMap::new(),
            tool_mapping: DashMap::new(),
            resource_mapping: DashMap::new(),
            prompt_mapping: DashMap::new(),
        }
    }

    /// Register and connect to an MCP server
    pub async fn register_server(
        &self,
        name: impl Into<String>,
        config: TransportConfig,
    ) -> Result<McpServerInfo, McpError> {
        let name = name.into();

        // Create transport based on config
        let transport: Box<dyn super::transport::McpTransport> = match config {
            TransportConfig::Stdio { command, args, env } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                Box::new(StdioTransport::spawn_with_env(&command, &args_refs, &env).await?)
            }
            TransportConfig::Http { base_url, headers } => {
                let http_config = HttpTransportConfig::new(&base_url);
                let http_config = headers.into_iter().fold(http_config, |cfg, (k, v)| {
                    cfg.with_header(k, v)
                });
                let mut transport = HttpTransport::new(http_config)?;
                transport.connect().await?;
                Box::new(transport)
            }
            TransportConfig::WebSocket { .. } => {
                return Err(McpError::Transport("WebSocket transport not yet implemented".into()));
            }
        };

        // Create and initialize client
        let client = Arc::new(McpClient::new(transport));
        let server_info = client.initialize().await?;

        // Store client
        self.clients.insert(name.clone(), client.clone());

        // Discover tools, resources, and prompts
        self.refresh_server_capabilities(&name, &client).await?;

        Ok(server_info)
    }

    /// Refresh capabilities for a server
    async fn refresh_server_capabilities(
        &self,
        name: &str,
        client: &Arc<McpClient>,
    ) -> Result<(), McpError> {
        // Get tools
        if let Ok(tools) = client.list_tools().await {
            for tool in tools {
                self.tool_mapping.insert(tool.name.clone(), name.to_string());
            }
        }

        // Get resources
        if let Ok(resources) = client.list_resources().await {
            for resource in resources {
                self.resource_mapping
                    .insert(resource.uri.clone(), name.to_string());
            }
        }

        // Get prompts
        if let Ok(prompts) = client.list_prompts().await {
            for prompt in prompts {
                self.prompt_mapping
                    .insert(prompt.name.clone(), name.to_string());
            }
        }

        Ok(())
    }

    /// Unregister and disconnect from an MCP server
    pub async fn unregister_server(&self, name: &str) -> Result<(), McpError> {
        if let Some((_, client)) = self.clients.remove(name) {
            // Remove mappings for this server
            self.tool_mapping.retain(|_, v| v != name);
            self.resource_mapping.retain(|_, v| v != name);
            self.prompt_mapping.retain(|_, v| v != name);

            // Close the client
            client.close().await?;
        }
        Ok(())
    }

    /// Get a client by name
    pub fn get_client(&self, name: &str) -> Option<Arc<McpClient>> {
        self.clients.get(name).map(|c| c.clone())
    }

    /// Get all server names
    pub fn server_names(&self) -> Vec<String> {
        self.clients.iter().map(|e| e.key().clone()).collect()
    }

    /// Get all available tools across all servers
    pub async fn all_tools(&self) -> Vec<McpTool> {
        let mut tools = Vec::new();
        for entry in self.clients.iter() {
            if let Ok(t) = entry.value().list_tools().await {
                tools.extend(t);
            }
        }
        tools
    }

    /// Get all available resources across all servers
    pub async fn all_resources(&self) -> Vec<McpResource> {
        let mut resources = Vec::new();
        for entry in self.clients.iter() {
            if let Ok(r) = entry.value().list_resources().await {
                resources.extend(r);
            }
        }
        resources
    }

    /// Get all available prompts across all servers
    pub async fn all_prompts(&self) -> Vec<McpPrompt> {
        let mut prompts = Vec::new();
        for entry in self.clients.iter() {
            if let Ok(p) = entry.value().list_prompts().await {
                prompts.extend(p);
            }
        }
        prompts
    }

    /// Call a tool by name
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String, McpError> {
        let server_name = self
            .tool_mapping
            .get(name)
            .map(|e| e.value().clone())
            .ok_or_else(|| McpError::ToolNotFound(name.to_string()))?;

        let client = self
            .clients
            .get(&server_name)
            .map(|e| e.clone())
            .ok_or_else(|| McpError::Connection(format!("Server {} not found", server_name)))?;

        let result = client.call_tool(name, arguments).await?;

        // Convert result to string
        let text = result
            .content
            .iter()
            .filter_map(|c| match c {
                super::types::McpContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        if result.is_error {
            Err(McpError::Server {
                code: -1,
                message: text,
            })
        } else {
            Ok(text)
        }
    }

    /// Read a resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<String, McpError> {
        let server_name = self
            .resource_mapping
            .get(uri)
            .map(|e| e.value().clone())
            .ok_or_else(|| McpError::ResourceNotFound(uri.to_string()))?;

        let client = self
            .clients
            .get(&server_name)
            .map(|e| e.clone())
            .ok_or_else(|| McpError::Connection(format!("Server {} not found", server_name)))?;

        let content = client.read_resource(uri).await?;

        content
            .text
            .ok_or_else(|| McpError::ResourceNotFound(uri.to_string()))
    }

    /// Convert MCP tools to Sage tools
    pub async fn as_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools = Vec::new();

        for entry in self.tool_mapping.iter() {
            let tool_name = entry.key().clone();
            let server_name = entry.value().clone();

            if let Some(client) = self.clients.get(&server_name) {
                // Find the tool definition
                if let Some(mcp_tool) = client
                    .cached_tools()
                    .await
                    .into_iter()
                    .find(|t| t.name == tool_name)
                {
                    let adapter = McpToolAdapter {
                        client: client.clone(),
                        tool: mcp_tool,
                    };
                    tools.push(Arc::new(adapter) as Arc<dyn Tool>);
                }
            }
        }

        tools
    }

    /// Close all connections
    pub async fn close_all(&self) -> Result<(), McpError> {
        for entry in self.clients.iter() {
            entry.value().close().await?;
        }
        self.clients.clear();
        self.tool_mapping.clear();
        self.resource_mapping.clear();
        self.prompt_mapping.clear();
        Ok(())
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter to use MCP tools as Sage tools
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool: McpTool,
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.tool.name
    }

    fn description(&self) -> &str {
        self.tool.description.as_deref().unwrap_or("")
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.tool.name.clone(),
            description: self.tool.description.clone().unwrap_or_default(),
            parameters: self.tool.input_schema.clone(),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, crate::tools::base::ToolError> {
        let arguments: Value = call
            .arguments
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        match self.client.call_tool(&self.tool.name, arguments).await {
            Ok(result) => {
                let text = result
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        super::types::McpContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if result.is_error {
                    Ok(ToolResult::error(&call.id, &self.tool.name, text))
                } else {
                    Ok(ToolResult::success(&call.id, &self.tool.name, text))
                }
            }
            Err(e) => Err(crate::tools::base::ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = McpRegistry::new();
        assert!(registry.server_names().is_empty());
    }

    #[test]
    fn test_transport_config() {
        let config = TransportConfig::stdio("echo", vec!["hello".to_string()]);
        assert!(matches!(config, TransportConfig::Stdio { .. }));
    }
}
