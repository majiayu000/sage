//! MCP protocol operations (tools, resources, prompts)

use super::super::error::McpError;
use super::super::protocol::methods;
use super::super::types::{
    McpPrompt, McpPromptMessage, McpResource, McpResourceContent, McpTool, McpToolResult,
};
use super::McpClient;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::instrument;

impl McpClient {
    /// List available tools
    #[instrument(skip(self), level = "debug")]
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        self.fetch_tools().await
    }

    pub(crate) async fn list_tools_uncached(&self) -> Result<Vec<McpTool>, McpError> {
        self.fetch_tools().await
    }

    async fn fetch_tools(&self) -> Result<Vec<McpTool>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::TOOLS_LIST, None).await?;

        decode_required_array(methods::TOOLS_LIST, &result, "tools")
    }

    /// Call a tool with timeout
    #[instrument(skip(self, arguments), fields(tool_name = %name))]
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult, McpError> {
        self.ensure_initialized().await?;

        let params = json!({
            "name": name,
            "arguments": arguments
        });

        // When the registry has installed a trusted-tool allowlist, refuse tools
        // that were later skipped for trust-baseline drift. Direct clients leave
        // the allowlist inactive so initialize → list_tools → call_tool works.
        //
        // Authorize and dispatch under transport → tools locks so listChanged
        // revoke (same order) cannot race a stale send. Await the response after
        // releasing those locks. Reject only when this tool is no longer
        // authorized: a client-wide generation bump from an unrelated tool-set
        // change must not turn a still-allowed call into tool_not_found (which
        // encourages duplicate retries after the remote side effect).
        // Reservation of the command-queue slot happens before the locks so a
        // full bounded channel cannot deadlock against listChanged draining.
        if self.trusted_tool_allowlist_active() {
            let response_receiver = self.begin_authorized_tool_call(name, params).await?;
            let result: McpToolResult = self.finish_call(response_receiver).await?;
            if !self
                .tools()
                .read()
                .await
                .iter()
                .any(|tool| tool.name == name)
            {
                return Err(McpError::tool_not_found(name.to_string()));
            }
            return Ok(result);
        }

        let result: McpToolResult = self.call(methods::TOOLS_CALL, Some(params)).await?;
        Ok(result)
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::RESOURCES_LIST, None).await?;

        let resources: Vec<McpResource> =
            decode_required_array(methods::RESOURCES_LIST, &result, "resources")?;

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
            decode_required_array(methods::RESOURCES_READ, &result, "contents")?;

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
            decode_required_array(methods::PROMPTS_LIST, &result, "prompts")?;

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
            decode_required_array(methods::PROMPTS_GET, &result, "messages")?;

        Ok(messages)
    }

    /// Ping the server
    pub async fn ping(&self) -> Result<(), McpError> {
        let _: Value = self.call(methods::PING, None).await?;
        Ok(())
    }

    /// Refresh server listings.
    ///
    /// Tool listings are fetched but are not written into the shared tool cache;
    /// registry refresh owns trust filtering before model-visible tools are cached.
    pub async fn refresh_caches(&self) -> Result<(), McpError> {
        self.list_tools().await?;
        self.list_resources().await?;
        self.list_prompts().await?;
        Ok(())
    }
}

fn decode_required_array<T>(method: &str, result: &Value, field: &str) -> Result<Vec<T>, McpError>
where
    T: DeserializeOwned,
{
    let value = result.get(field).ok_or_else(|| {
        McpError::schema(format!(
            "MCP response for '{method}' is missing required array field '{field}'"
        ))
    })?;
    if !value.is_array() {
        return Err(McpError::schema(format!(
            "MCP response for '{method}' field '{field}' must be an array"
        )));
    }
    serde_json::from_value(value.clone()).map_err(|err| {
        McpError::schema(format!(
            "Failed to decode MCP response for '{method}' field '{field}': {err}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mcp_tools_list_schema_decode_rejects_missing_tools() {
        let err = decode_required_array::<McpTool>(methods::TOOLS_LIST, &json!({}), "tools")
            .expect_err("missing tools should be schema error");

        assert!(matches!(err, McpError::Schema { .. }));
    }

    #[test]
    fn mcp_tools_list_schema_decode_rejects_non_array_tools() {
        let err =
            decode_required_array::<McpTool>(methods::TOOLS_LIST, &json!({"tools": {}}), "tools")
                .expect_err("object tools should be schema error");

        assert!(matches!(err, McpError::Schema { .. }));
    }
}
