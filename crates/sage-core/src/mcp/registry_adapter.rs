//! Adapter that wraps an MCP tool as a Sage Tool.

use super::client::McpClient;
use super::deferred_tools::namespaced_tool_name as build_namespaced_tool_name;
use super::types::McpTool;
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use crate::types::tool::ToolParameter;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

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
