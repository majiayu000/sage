//! MCP registry for managing multiple MCP servers
//!
//! Provides centralized management of MCP servers and their tools.

use super::client::McpClient;
use super::deferred_tools::{
    McpDeferredToolIndex, namespaced_tool_name as build_namespaced_tool_name,
};
use super::error::McpError;
use super::runtime_status::{McpRuntimeState, McpServerRuntimeStatus, McpToolDiscoveryState};
use super::source::MergedMcpServerSource;
use super::transport::{HttpTransport, HttpTransportConfig, StdioTransport, TransportConfig};
use super::types::{McpPrompt, McpResource, McpServerInfo, McpTool};
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use crate::types::tool::ToolParameter;
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct ToolRoute {
    pub(crate) server_name: String,
    pub(crate) remote_name: String,
}

/// Registry for managing MCP servers and their capabilities
pub struct McpRegistry {
    /// Configured MCP sources by server name
    pub(crate) sources: DashMap<String, MergedMcpServerSource>,
    /// Runtime status by server name
    pub(crate) statuses: DashMap<String, McpServerRuntimeStatus>,
    /// Connected MCP clients by name
    pub(crate) clients: DashMap<String, Arc<McpClient>>,
    /// Namespaced tool to route mapping
    pub(crate) tool_mapping: DashMap<String, ToolRoute>,
    /// Resource to client mapping
    pub(crate) resource_mapping: DashMap<String, String>,
    /// Prompt to client mapping
    pub(crate) prompt_mapping: DashMap<String, String>,
    /// Deferred searchable tool metadata
    pub(crate) deferred_tools: RwLock<McpDeferredToolIndex>,
}

impl McpRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            sources: DashMap::new(),
            statuses: DashMap::new(),
            clients: DashMap::new(),
            tool_mapping: DashMap::new(),
            resource_mapping: DashMap::new(),
            prompt_mapping: DashMap::new(),
            deferred_tools: RwLock::new(McpDeferredToolIndex::new()),
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
                let http_config = headers
                    .into_iter()
                    .fold(http_config, |cfg, (k, v)| cfg.with_header(k, v));
                let mut transport = HttpTransport::new(http_config)?;
                transport.connect().await?;
                Box::new(transport)
            }
            TransportConfig::WebSocket { .. } => {
                return Err(McpError::transport(
                    "WebSocket transport not yet implemented",
                ));
            }
        };

        // Create and initialize client
        let client = Arc::new(McpClient::new(transport));
        let server_info = client.initialize().await?;

        // Store client
        self.clients.insert(name.clone(), client.clone());

        // Discover tools, resources, and prompts
        if let Err(error) = self.refresh_server_capabilities(&name, &client).await {
            self.clients.remove(&name);
            if let Err(close_error) = client.close().await {
                tracing::debug!(
                    "Failed to close MCP client '{}' after capability error: {}",
                    name,
                    close_error
                );
            }
            self.tool_mapping
                .retain(|_, route| route.server_name != name);
            self.resource_mapping.retain(|_, v| v != &name);
            self.prompt_mapping.retain(|_, v| v != &name);
            self.deferred_tools
                .write()
                .mark_server(name.clone(), McpToolDiscoveryState::SchemaError);
            return Err(error);
        }

        Ok(server_info)
    }

    /// Unregister and disconnect from an MCP server
    pub async fn unregister_server(&self, name: &str) -> Result<(), McpError> {
        if let Some((_, client)) = self.clients.remove(name) {
            // Remove mappings for this server
            self.tool_mapping
                .retain(|_, route| route.server_name != name);
            self.resource_mapping.retain(|_, v| v != name);
            self.prompt_mapping.retain(|_, v| v != name);

            // Close the client
            client.close().await?;
        }
        self.deferred_tools.write().mark_server_stale(name);
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
        let route = match self
            .tool_mapping
            .get(name)
            .map(|entry| entry.value().clone())
        {
            Some(route) => route,
            None => {
                if let Some(status) = self.status_for_tool_name(name) {
                    if status.auth_blocks_tools() {
                        if let Some(prompt) = status.auth.prompt {
                            return Err(McpError::auth_required(status.server_id, prompt));
                        }
                    }
                    if matches!(status.state, McpRuntimeState::Disabled) {
                        return Err(McpError::disabled(status.server_id));
                    }
                }
                return Err(McpError::tool_not_found(name.to_string()));
            }
        };

        let client = self
            .clients
            .get(&route.server_name)
            .map(|e| e.clone())
            .ok_or_else(|| {
                McpError::connection(format!("Server {} not found", route.server_name))
            })?;

        let result = client.call_tool(&route.remote_name, arguments).await?;

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
            Err(McpError::server(-1, text))
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
            .ok_or_else(|| McpError::resource_not_found(uri.to_string()))?;

        let client = self
            .clients
            .get(&server_name)
            .map(|e| e.clone())
            .ok_or_else(|| McpError::connection(format!("Server {} not found", server_name)))?;

        let content = client.read_resource(uri).await?;

        content
            .text
            .ok_or_else(|| McpError::resource_not_found(uri.to_string()))
    }

    /// Convert MCP tools to Sage tools
    pub async fn as_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools = Vec::new();

        for client_entry in self.clients.iter() {
            let server_name = client_entry.key().clone();
            let client = client_entry.value().clone();
            for mcp_tool in client.cached_tools().await {
                let adapter = McpToolAdapter::new(mcp_tool, client.clone(), server_name.clone());
                tools.push(Arc::new(adapter) as Arc<dyn Tool>);
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
        *self.deferred_tools.write() = McpDeferredToolIndex::new();
        Ok(())
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter that wraps an MCP tool as a Sage Tool.
/// Canonical definition — sage-tools re-exports this.
pub struct McpToolAdapter {
    exposed_name: String,
    mcp_tool: McpTool,
    client: Arc<McpClient>,
    server_name: String,
}

impl McpToolAdapter {
    pub fn new(mcp_tool: McpTool, client: Arc<McpClient>, server_name: String) -> Self {
        let exposed_name = Self::namespaced_tool_name(&server_name, &mcp_tool.name);
        Self {
            exposed_name,
            mcp_tool,
            client,
            server_name,
        }
    }

    pub fn namespaced_tool_name(server_name: &str, remote_tool_name: &str) -> String {
        build_namespaced_tool_name(server_name, remote_tool_name)
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    pub fn mcp_tool(&self) -> &McpTool {
        &self.mcp_tool
    }

    fn convert_schema(&self) -> Vec<ToolParameter> {
        let mut params = Vec::new();
        let input_schema = &self.mcp_tool.input_schema;

        if input_schema.is_null() {
            return params;
        }

        if let Some(properties) = input_schema.get("properties").and_then(|p| p.as_object()) {
            let required_fields: Vec<String> = input_schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            for (name, schema) in properties {
                let description = schema
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                let is_required = required_fields.contains(name);
                let param_type = schema
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string");

                let param = match (is_required, param_type) {
                    (true, "string") => ToolParameter::string(name, &description),
                    (true, "integer") | (true, "number") => {
                        ToolParameter::number(name, &description)
                    }
                    (true, "boolean") => ToolParameter::boolean(name, &description),
                    (true, _) => ToolParameter::string(name, &description),
                    (false, "string") => ToolParameter::optional_string(name, &description),
                    (false, _) => ToolParameter::optional_string(name, &description),
                };

                params.push(param);
            }
        }

        params
    }

    fn convert_result(
        &self,
        call: &ToolCall,
        mcp_result: super::types::McpToolResult,
    ) -> ToolResult {
        let output = mcp_result
            .content
            .iter()
            .map(|c| match c {
                super::types::McpContent::Text { text } => text.clone(),
                super::types::McpContent::Image { .. } => "[Image content]".to_string(),
                super::types::McpContent::Resource { .. } => "[Resource reference]".to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        if mcp_result.is_error {
            ToolResult::error(
                &call.id,
                self.name(),
                format!("MCP tool execution failed: {}", output),
            )
        } else {
            ToolResult::success(&call.id, self.name(), output)
        }
    }
}

impl std::fmt::Debug for McpToolAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpToolAdapter")
            .field("name", &self.exposed_name)
            .field("remote_name", &self.mcp_tool.name)
            .field("server", &self.server_name)
            .finish()
    }
}

impl Clone for McpToolAdapter {
    fn clone(&self) -> Self {
        Self {
            exposed_name: self.exposed_name.clone(),
            mcp_tool: self.mcp_tool.clone(),
            client: Arc::clone(&self.client),
            server_name: self.server_name.clone(),
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.exposed_name
    }

    fn description(&self) -> &str {
        self.mcp_tool.description.as_deref().unwrap_or("MCP tool")
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), self.convert_schema())
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, crate::tools::base::ToolError> {
        let arguments: Value = serde_json::to_value(&call.arguments).map_err(|e| {
            crate::tools::base::ToolError::InvalidArguments(format!(
                "Failed to serialize arguments: {}",
                e
            ))
        })?;

        let result = self
            .client
            .call_tool(&self.mcp_tool.name, arguments)
            .await
            .map_err(|e| {
                crate::tools::base::ToolError::ExecutionFailed(format!(
                    "MCP tool call failed: {}",
                    e
                ))
            })?;

        Ok(self.convert_result(call, result))
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
    fn test_namespaced_tool_name() {
        let namespaced = McpToolAdapter::namespaced_tool_name("filesystem-server", "Read File");
        assert_eq!(namespaced, "mcp__filesystem_server__read_file");
    }

    #[test]
    fn test_transport_config() {
        let config = TransportConfig::stdio("echo", vec!["hello".to_string()]);
        assert!(matches!(config, TransportConfig::Stdio { .. }));
    }
}
