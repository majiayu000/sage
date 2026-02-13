//! MCP Servers Management Tool
//!
//! Provides a tool for viewing and managing MCP server connections within the agent.

use anyhow::anyhow;
use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use std::sync::Arc;
use tokio::sync::OnceCell;

use super::registry::SharedMcpToolRegistry;

/// Global MCP tool registry instance
static GLOBAL_MCP_REGISTRY: OnceCell<SharedMcpToolRegistry> = OnceCell::const_new();

/// Initialize the global MCP tool registry
pub async fn init_global_mcp_registry(registry: SharedMcpToolRegistry) -> anyhow::Result<()> {
    GLOBAL_MCP_REGISTRY
        .set(registry)
        .map_err(|_| anyhow!("MCP registry already initialized"))
}

/// Get the global MCP tool registry
pub fn get_global_mcp_registry() -> Option<SharedMcpToolRegistry> {
    GLOBAL_MCP_REGISTRY.get().cloned()
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
                let statuses = registry.server_statuses().await;

                if statuses.is_empty() {
                    "No MCP servers configured.".to_string()
                } else {
                    let mut output = format!("MCP Servers ({} total):\n\n", statuses.len());
                    for status in statuses {
                        let status_icon = if status.connected { "✓" } else { "✗" };
                        output.push_str(&format!(
                            "{} {} - {} tool(s){}\n",
                            status_icon,
                            status.name,
                            status.tool_count,
                            if let Some(err) = &status.error {
                                format!(" (Error: {})", err)
                            } else {
                                String::new()
                            }
                        ));
                    }
                    output
                }
            }

            "tools" => {
                let server_name = call.get_string("server");

                let tools_by_server = registry.tool_names_by_server().await;

                if tools_by_server.is_empty() {
                    "No MCP tools available. Connect to an MCP server first.".to_string()
                } else if let Some(server) = server_name {
                    // Show tools from specific server
                    if let Some(tools) = tools_by_server.get(&server) {
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
                    for (server, tools) in &tools_by_server {
                        output.push_str(&format!("From '{}':\n", server));
                        for tool in tools {
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

                let statuses = registry.server_statuses().await;

                if let Some(status) = statuses.iter().find(|s| s.name == server_name) {
                    format!(
                        "Server: {}\n\
                         Connected: {}\n\
                         Tools: {}\n\
                         {}",
                        status.name,
                        if status.connected { "Yes" } else { "No" },
                        status.tool_count,
                        if let Some(err) = &status.error {
                            format!("Error: {}", err)
                        } else {
                            String::new()
                        }
                    )
                } else {
                    format!("Server '{}' not found.", server_name)
                }
            }

            "refresh" => {
                let tool_count = registry.tool_count().await;
                let server_count = registry.server_count().await;

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
        Some(registry) => registry.all_tools().await,
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_servers_tool_list_no_registry() {
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
