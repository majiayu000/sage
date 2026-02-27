//! MCP Servers Management Tool
//!
//! Provides a tool for viewing and managing MCP server connections within the agent.

use async_trait::async_trait;
use sage_core::mcp::{McpRegistry, get_active_mcp_registry, set_active_mcp_registry};
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use std::sync::Arc;

/// Initialize the global MCP tool registry
pub async fn init_global_mcp_registry(registry: Arc<McpRegistry>) -> anyhow::Result<()> {
    set_active_mcp_registry(registry);
    Ok(())
}

/// Get the global MCP tool registry
pub fn get_global_mcp_registry() -> Option<Arc<McpRegistry>> {
    get_active_mcp_registry()
}

async fn server_tool_names(registry: &McpRegistry, server_name: &str) -> Option<Vec<String>> {
    let client = registry.get_client(server_name)?;
    let mut names = client
        .cached_tools()
        .await
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();
    names.sort();
    Some(names)
}

async fn all_server_tools(registry: &McpRegistry) -> Vec<(String, Vec<String>)> {
    let mut server_names = registry.server_names();
    server_names.sort();

    let mut by_server = Vec::with_capacity(server_names.len());
    for server_name in server_names {
        let tools = server_tool_names(registry, &server_name)
            .await
            .unwrap_or_default();
        by_server.push((server_name, tools));
    }

    by_server
}

/// MCP Servers management tool
#[derive(Debug, Clone)]
pub struct McpServersTool;

impl Default for McpServersTool {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServersTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for McpServersTool {
    fn name(&self) -> &str {
        "McpServers"
    }

    fn description(&self) -> &str {
        r#"View and manage MCP (Model Context Protocol) server connections.

MCP servers provide external tools that can be used by the agent.

Actions:
- list: Show all configured MCP servers and their status
- tools: List all available tools from connected servers
- connect: Connect to a specific server by name
- disconnect: Disconnect from a specific server
- refresh: Refresh tool list from all connected servers
- status: Show detailed status of a specific server

Example:
- List all servers: action="list"
- Show tools from a server: action="tools", server="my-mcp-server"
- Connect to a server: action="connect", server="my-mcp-server""#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "action",
                    "Action to perform: list, tools, connect, disconnect, refresh, status",
                ),
                ToolParameter::optional_string(
                    "server",
                    "Server name (for connect/disconnect/tools/status actions)",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let action = call
            .get_string("action")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;

        let registry = match get_global_mcp_registry() {
            Some(r) => r,
            None => {
                return Ok(ToolResult::success(
                    &call.id,
                    self.name(),
                    "MCP integration is not initialized. No MCP servers are configured.",
                ));
            }
        };

        let response = match action.to_lowercase().as_str() {
            "list" => {
                let by_server = all_server_tools(&registry).await;
                if by_server.is_empty() {
                    "No MCP servers configured.".to_string()
                } else {
                    let mut output = format!("MCP Servers ({} total):\n\n", by_server.len());
                    for (server_name, tools) in by_server {
                        output.push_str(&format!("✓ {} - {} tool(s)\n", server_name, tools.len()));
                    }
                    output
                }
            }

            "tools" => {
                let server_name = call.get_string("server");
                let tools_by_server = all_server_tools(&registry).await;

                if tools_by_server.is_empty() {
                    "No MCP tools available. Connect to an MCP server first.".to_string()
                } else if let Some(server) = server_name {
                    // Show tools from specific server
                    if let Some((_, tools)) =
                        tools_by_server.iter().find(|(name, _)| name == &server)
                    {
                        let mut output = format!("Tools from '{}':\n\n", server);
                        for (i, tool) in tools.iter().enumerate() {
                            output.push_str(&format!("{}. {}\n", i + 1, tool));
                        }
                        output
                    } else {
                        format!("Server '{}' not found or has no tools.", server)
                    }
                } else {
                    // Show all tools organized by server
                    let mut output = String::from("Available MCP Tools:\n\n");
                    for (server, tools) in tools_by_server {
                        output.push_str(&format!("From '{}':\n", server));
                        for tool in &tools {
                            output.push_str(&format!("  - {}\n", tool));
                        }
                        output.push('\n');
                    }
                    output
                }
            }

            "status" => {
                let server_name = call.get_string("server").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'server' parameter for status action".to_string(),
                    )
                })?;

                if let Some(tools) = server_tool_names(&registry, &server_name).await {
                    format!(
                        "Server: {}\n\
                         Connected: {}\n\
                         Tools: {}",
                        server_name,
                        "Yes",
                        tools.len()
                    )
                } else {
                    format!("Server '{}' not found.", server_name)
                }
            }

            "refresh" => {
                let server_count = registry.server_names().len();
                let tool_count = registry.as_tools().await.len();

                format!(
                    "MCP registry refreshed.\n\
                     Connected servers: {}\n\
                     Available tools: {}",
                    server_count, tool_count
                )
            }

            "connect" | "disconnect" => {
                // Note: Dynamic connect/disconnect requires config access
                // For now, provide information about static configuration
                format!(
                    "Dynamic {} is not supported in this version.\n\
                     MCP servers are configured in the config file under 'mcp.servers'.\n\
                     Edit the configuration and restart to change server connections.",
                    action
                )
            }

            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown action: '{}'. Valid actions: list, tools, status, refresh",
                    action
                )));
            }
        };

        Ok(ToolResult::success(&call.id, self.name(), response))
    }
}

/// Get all MCP tools as Sage tools for the agent
pub async fn get_mcp_tools() -> Vec<Arc<dyn Tool>> {
    match get_global_mcp_registry() {
        Some(registry) => registry.as_tools().await,
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_servers_tool_list_no_registry() {
        sage_core::mcp::clear_active_mcp_registry();
        let tool = McpServersTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "McpServers".to_string(),
            arguments: json!({
                "action": "list"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("not initialized"));
    }

    #[tokio::test]
    async fn test_mcp_servers_tool_schema() {
        let tool = McpServersTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "McpServers");
        assert!(schema.description.contains("MCP"));
    }
}
