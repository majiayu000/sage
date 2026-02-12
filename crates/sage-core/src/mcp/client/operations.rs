//! MCP protocol operations (tools, resources, prompts)

use super::McpClient;
use super::super::error::McpError;
use super::super::protocol::methods;
use super::super::types::{
    McpPrompt, McpPromptMessage, McpResource, McpResourceContent, McpTool, McpToolResult,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::instrument;

impl McpClient {
    /// List available tools
    #[instrument(skip(self), level = "debug")]
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::TOOLS_LIST, None).await?;

        let tools: Vec<McpTool> =
            serde_json::from_value(result["tools"].clone()).unwrap_or_default();

        *self.tools().write().await = tools.clone();
        Ok(tools)
    }

    /// Call a tool with timeout
    #[instrument(skip(self, arguments), fields(tool_name = %name))]
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult, McpError> {
        self.ensure_initialized().await?;

        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let result: McpToolResult = self.call(methods::TOOLS_CALL, Some(params)).await?;
        Ok(result)
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::RESOURCES_LIST, None).await?;

        let resources: Vec<McpResource> =
            serde_json::from_value(result["resources"].clone()).unwrap_or_default();

        *self.resources().write().await = resources.clone();
        Ok(resources)
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<McpResourceContent, McpError> {
        self.ensure_initialized().await?;

        let params = json!({
            "uri": uri
        });

        let result: Value = self.call(methods::RESOURCES_READ, Some(params)).await?;

        // The result should contain "contents" array
        let contents: Vec<McpResourceContent> =
            serde_json::from_value(result["contents"].clone()).unwrap_or_default();

        contents
            .into_iter()
            .next()
            .ok_or_else(|| McpError::resource_not_found(uri.to_string()))
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::PROMPTS_LIST, None).await?;

        let prompts: Vec<McpPrompt> =
            serde_json::from_value(result["prompts"].clone()).unwrap_or_default();

        *self.prompts().write().await = prompts.clone();
        Ok(prompts)
    }

    /// Get a prompt with optional arguments
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<Vec<McpPromptMessage>, McpError> {
        self.ensure_initialized().await?;

        let params = json!({
            "name": name,
            "arguments": arguments.unwrap_or_default()
        });

        let result: Value = self.call(methods::PROMPTS_GET, Some(params)).await?;

        let messages: Vec<McpPromptMessage> =
            serde_json::from_value(result["messages"].clone()).unwrap_or_default();

        Ok(messages)
    }

    /// Ping the server
    pub async fn ping(&self) -> Result<(), McpError> {
        let _: Value = self.call(methods::PING, None).await?;
        Ok(())
    }

    /// Refresh all caches (tools, resources, prompts)
    pub async fn refresh_caches(&self) -> Result<(), McpError> {
        self.list_tools().await?;
        self.list_resources().await?;
        self.list_prompts().await?;
        Ok(())
    }
}
