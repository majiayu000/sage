//! MCP Servers Management Tool
//!
//! Provides a tool for viewing and managing MCP server connections within the agent.

use async_trait::async_trait;
use sage_core::mcp::{
    McpDeferredTool, McpRegistry, McpServerRuntimeStatus, get_active_mcp_registry,
    set_active_mcp_registry,
};
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
- tools: List/search deferred tools without connecting every server
- connect: Connect to a specific server by name
- disconnect: Disconnect from a specific server
- retry: Retry a failed server
- refresh: Refresh tool list from all connected servers
- status: Show detailed status of a specific server

Example:
- List all servers: action="list"
- Show tools from a server: action="tools", server="my-mcp-server"
- Search deferred tools: action="tools", query="filesystem"
- Connect to a server: action="connect", server="my-mcp-server""#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "action",
                    "Action to perform: list, tools, connect, disconnect, retry, refresh, status",
                ),
                ToolParameter::optional_string(
                    "server",
                    "Server name (for connect/disconnect/tools/status actions)",
                ),
                ToolParameter::optional_string("query", "Search query for deferred MCP tools"),
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
                let statuses = registry.runtime_statuses();
                if statuses.is_empty() {
                    "No MCP servers configured.".to_string()
                } else {
                    let mut output = format!("MCP Servers ({} total):\n\n", statuses.len());
                    for status in statuses {
                        output.push_str(&format_status_line(&status, &registry));
                    }
                    output
                }
            }

            "tools" => {
                let server_name = call.get_string("server");
                let query = call.get_string("query").unwrap_or_default();
                let tools = if query.trim().is_empty() {
                    registry.deferred_tools()
                } else {
                    registry.search_deferred_tools(&query)
                };

                if tools.is_empty() {
                    "No MCP tools available. Connect to an MCP server first.".to_string()
                } else if let Some(server) = server_name {
                    let filtered = tools
                        .into_iter()
                        .filter(|tool| tool.server_id == server)
                        .collect::<Vec<_>>();
                    if !filtered.is_empty() {
                        let mut output = format!("Tools from '{}':\n\n", server);
                        append_deferred_tools(&mut output, &filtered);
                        output
                    } else {
                        format!("Server '{}' not found or has no tools.", server)
                    }
                } else {
                    let mut output = String::from("Available MCP Tools:\n\n");
                    append_deferred_tools(&mut output, &tools);
                    output
                }
            }

            "status" => {
                let server_name = call.get_string("server").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'server' parameter for status action".to_string(),
                    )
                })?;

                if let Some(status) = registry.server_runtime_status(&server_name) {
                    format_detailed_status(&status, &registry)
                } else if let Some(tools) = server_tool_names(&registry, &server_name).await {
                    format!(
                        "Server: {server_name}\nState: connected\nTools: {}",
                        tools.len()
                    )
                } else {
                    format!("Server '{}' not found.", server_name)
                }
            }

            "refresh" => {
                let server_count = registry.server_names().len();
                let tool_count = registry.deferred_tools().len();

                format!(
                    "MCP registry refreshed.\n\
                     Connected servers: {}\n\
                     Deferred tools: {}",
                    server_count, tool_count
                )
            }

            "connect" | "disconnect" | "retry" => {
                let server_name = call.get_string("server").ok_or_else(|| {
                    ToolError::InvalidArguments(format!(
                        "Missing 'server' parameter for {} action",
                        action
                    ))
                })?;
                let result = match action.to_lowercase().as_str() {
                    "connect" => registry.connect_configured_server(&server_name).await,
                    "disconnect" => registry.disconnect_configured_server(&server_name).await,
                    "retry" => registry.retry_configured_server(&server_name).await,
                    _ => unreachable!(),
                };
                match result {
                    Ok(result) => format_detailed_status(&result.status, &registry),
                    Err(error) => format!("MCP {action} failed for '{server_name}': {error}"),
                }
            }

            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown action: '{}'. Valid actions: list, tools, status, refresh, connect, disconnect, retry",
                    action
                )));
            }
        };

        Ok(ToolResult::success(&call.id, self.name(), response))
    }
}

fn format_status_line(status: &McpServerRuntimeStatus, registry: &McpRegistry) -> String {
    let tools = registry
        .deferred_tools()
        .into_iter()
        .filter(|tool| tool.server_id == status.server_id)
        .count();
    format!(
        "- {} - state={:?}, auth={:?}, source={:?}, tools={}\n",
        status.server_id, status.state, status.auth.state, status.source.kind, tools
    )
}

fn format_detailed_status(status: &McpServerRuntimeStatus, registry: &McpRegistry) -> String {
    let mut output = format!(
        "Server: {}\nState: {:?}\nAuth: {:?}\nSource: {:?}\nEnabled: {}\n",
        status.server_id, status.state, status.auth.state, status.source.kind, status.enabled
    );
    if let Some(prompt) = &status.auth.prompt {
        output.push_str(&format!("Authorization: {}\n", prompt.message));
        if let Some(url) = &prompt.authorization_url {
            output.push_str(&format!("Authorization URL: {}\n", url));
        }
    }
    if let Some(error) = &status.last_error {
        output.push_str(&format!("Last error: {} ({})\n", error.message, error.code));
    }
    let tools = registry
        .deferred_tools()
        .into_iter()
        .filter(|tool| tool.server_id == status.server_id)
        .collect::<Vec<_>>();
    output.push_str(&format!("Deferred tools: {}\n", tools.len()));
    output
}

fn append_deferred_tools(output: &mut String, tools: &[McpDeferredTool]) {
    for (index, tool) in tools.iter().enumerate() {
        output.push_str(&format!(
            "{}. {} (server={}, freshness={:?})\n",
            index + 1,
            tool.name,
            tool.server_id,
            tool.freshness
        ));
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
    use sage_core::config::{McpAuthConfig, McpAuthKind, McpServerConfig};
    use sage_core::mcp::{McpServerSource, merge_mcp_sources};
    use serde_json::json;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
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

    #[tokio::test]
    #[serial]
    async fn test_mcp_servers_tool_status_reports_auth_required() {
        let registry = Arc::new(McpRegistry::new());
        let source_set = merge_mcp_sources([McpServerSource::direct(
            "secure",
            McpServerConfig::http("https://mcp.example.test").with_auth(McpAuthConfig {
                required: true,
                kind: McpAuthKind::OAuth,
                token_env: None,
                authorization_url: Some("https://auth.example.test/start".to_string()),
                scopes: Vec::new(),
            }),
            true,
        )])
        .expect("source set should merge");
        registry.apply_source_set(source_set);
        sage_core::mcp::set_active_mcp_registry(registry);

        let tool = McpServersTool::new();
        let call = ToolCall {
            id: "test-auth".to_string(),
            name: "McpServers".to_string(),
            arguments: json!({
                "action": "status",
                "server": "secure"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        let output = result.output.expect("status output");
        assert!(output.contains("AuthRequired"));
        assert!(output.contains("Authorization URL: https://auth.example.test/start"));
        sage_core::mcp::clear_active_mcp_registry();
    }
}
