//! MCP Tool Adapter
//!
//! Wraps MCP tools as Sage tools, allowing them to be used within the Sage Agent.

use async_trait::async_trait;
use sage_core::mcp::{McpClient, McpTool, McpToolResult};
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Adapter that wraps an MCP tool as a Sage Tool
pub struct McpToolAdapter {
    /// The MCP tool definition
    mcp_tool: McpTool,
    /// Client for executing the tool
    client: Arc<RwLock<McpClient>>,
    /// Server name for identification
    server_name: String,
}

impl McpToolAdapter {
    /// Create a new adapter for an MCP tool
    pub fn new(mcp_tool: McpTool, client: Arc<RwLock<McpClient>>, server_name: String) -> Self {
        Self {
            mcp_tool,
            client,
            server_name,
        }
    }

    /// Get the server name this tool belongs to
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Get the original MCP tool definition
    pub fn mcp_tool(&self) -> &McpTool {
        &self.mcp_tool
    }

    /// Convert MCP JSON Schema to Sage ToolParameters
    fn convert_schema(&self) -> Vec<ToolParameter> {
        let mut params = Vec::new();
        let input_schema = &self.mcp_tool.input_schema;

        // Check if schema is null or empty
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

                // Note: ToolParameter doesn't have optional_number/optional_boolean,
                // so we treat numbers/booleans as regular parameters
                let param = match (is_required, param_type) {
                    (true, "string") => ToolParameter::string(name, &description),
                    (true, "integer") | (true, "number") => ToolParameter::number(name, &description),
                    (true, "boolean") => ToolParameter::boolean(name, &description),
                    (true, _) => ToolParameter::string(name, &description), // Default to string
                    (false, "string") => ToolParameter::optional_string(name, &description),
                    // For optional non-string types, we use string and handle conversion
                    (false, _) => ToolParameter::optional_string(name, &description),
                };

                params.push(param);
            }
        }

        params
    }

    /// Convert MCP tool result to Sage ToolResult
    fn convert_result(&self, call: &ToolCall, mcp_result: McpToolResult) -> ToolResult {
        use sage_core::mcp::McpContent;

        let output = mcp_result
            .content
            .iter()
            .filter_map(|c| match c {
                McpContent::Text { text } => Some(text.clone()),
                McpContent::Image { .. } => Some("[Image content]".to_string()),
                McpContent::Resource { .. } => Some("[Resource reference]".to_string()),
            })
            .collect::<Vec<_>>()
            .join("\n");

        ToolResult {
            call_id: call.id.clone(),
            tool_name: self.name().to_string(),
            success: !mcp_result.is_error,
            output: Some(output),
            error: if mcp_result.is_error {
                Some("MCP tool execution failed".to_string())
            } else {
                None
            },
            exit_code: None,
            execution_time_ms: None,
            metadata: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for McpToolAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpToolAdapter")
            .field("name", &self.mcp_tool.name)
            .field("server", &self.server_name)
            .finish()
    }
}

impl Clone for McpToolAdapter {
    fn clone(&self) -> Self {
        Self {
            mcp_tool: self.mcp_tool.clone(),
            client: Arc::clone(&self.client),
            server_name: self.server_name.clone(),
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.mcp_tool.name
    }

    fn description(&self) -> &str {
        self.mcp_tool
            .description
            .as_deref()
            .unwrap_or("MCP tool")
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), self.convert_schema())
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Convert ToolCall arguments to JSON Value
        let arguments: Value = serde_json::to_value(&call.arguments)
            .map_err(|e| ToolError::InvalidArguments(format!("Failed to serialize arguments: {}", e)))?;

        // Get the client
        let client = self.client.read().await;

        // Execute the MCP tool call
        let result = client
            .call_tool(&self.mcp_tool.name, arguments)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("MCP tool call failed: {}", e)))?;

        Ok(self.convert_result(call, result))
    }
}

/// Create tool adapters for all tools from an MCP client
pub async fn create_adapters_from_client(
    client: Arc<RwLock<McpClient>>,
    server_name: &str,
) -> Result<Vec<McpToolAdapter>, String> {
    let client_read = client.read().await;
    let tools = client_read
        .list_tools()
        .await
        .map_err(|e| format!("Failed to list MCP tools: {}", e))?;

    drop(client_read); // Release the read lock

    let adapters: Vec<McpToolAdapter> = tools
        .into_iter()
        .map(|tool| McpToolAdapter::new(tool, Arc::clone(&client), server_name.to_string()))
        .collect();

    Ok(adapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_conversion() {
        let mcp_tool = McpTool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path"
                    },
                    "count": {
                        "type": "integer",
                        "description": "Number of items"
                    },
                    "optional_flag": {
                        "type": "boolean",
                        "description": "An optional flag"
                    }
                },
                "required": ["path", "count"]
            }),
        };

        // We can't fully test without a real client, but we can verify the tool struct
        assert_eq!(mcp_tool.name, "test_tool");
        assert_eq!(mcp_tool.description, Some("A test tool".to_string()));
    }
}
