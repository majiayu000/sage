//! MCP client implementation
//!
//! Provides a high-level client for communicating with MCP servers.

use super::error::McpError;
use super::protocol::{methods, McpMessage, McpNotification, McpRequest, McpResponse, RequestId, MCP_PROTOCOL_VERSION};
use super::transport::McpTransport;
use super::types::{
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, McpCapabilities,
    McpPrompt, McpPromptMessage, McpResource, McpResourceContent, McpServerInfo, McpTool,
    McpToolResult,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, RwLock};

/// MCP client for communicating with MCP servers
pub struct McpClient {
    /// Transport layer
    transport: Mutex<Box<dyn McpTransport>>,
    /// Server info
    server_info: RwLock<Option<McpServerInfo>>,
    /// Server capabilities
    capabilities: RwLock<McpCapabilities>,
    /// Cached tools
    tools: RwLock<Vec<McpTool>>,
    /// Cached resources
    resources: RwLock<Vec<McpResource>>,
    /// Cached prompts
    prompts: RwLock<Vec<McpPrompt>>,
    /// Request ID counter
    request_id: AtomicU64,
    /// Pending requests (id -> response channel)
    pending_requests: RwLock<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>,
    /// Whether initialized
    initialized: RwLock<bool>,
}

impl McpClient {
    /// Create a new MCP client with the given transport
    pub fn new(transport: Box<dyn McpTransport>) -> Self {
        Self {
            transport: Mutex::new(transport),
            server_info: RwLock::new(None),
            capabilities: RwLock::new(McpCapabilities::default()),
            tools: RwLock::new(Vec::new()),
            resources: RwLock::new(Vec::new()),
            prompts: RwLock::new(Vec::new()),
            request_id: AtomicU64::new(1),
            pending_requests: RwLock::new(HashMap::new()),
            initialized: RwLock::new(false),
        }
    }

    /// Initialize the MCP connection
    pub async fn initialize(&self) -> Result<McpServerInfo, McpError> {
        if *self.initialized.read().await {
            return Err(McpError::AlreadyInitialized);
        }

        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo::default(),
        };

        let result: InitializeResult = self
            .call(methods::INITIALIZE, Some(json!(params)))
            .await?;

        // Store server info and capabilities
        *self.server_info.write().await = Some(result.server_info.clone());
        *self.capabilities.write().await = result.capabilities;
        *self.initialized.write().await = true;

        // Send initialized notification
        self.notify(methods::INITIALIZED, None).await?;

        Ok(result.server_info)
    }

    /// Check if the client is initialized
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    /// Get server info
    pub async fn server_info(&self) -> Option<McpServerInfo> {
        self.server_info.read().await.clone()
    }

    /// Get server capabilities
    pub async fn capabilities(&self) -> McpCapabilities {
        self.capabilities.read().await.clone()
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::TOOLS_LIST, None).await?;

        let tools: Vec<McpTool> = serde_json::from_value(result["tools"].clone())
            .unwrap_or_default();

        *self.tools.write().await = tools.clone();
        Ok(tools)
    }

    /// Call a tool
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<McpToolResult, McpError> {
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

        let resources: Vec<McpResource> = serde_json::from_value(result["resources"].clone())
            .unwrap_or_default();

        *self.resources.write().await = resources.clone();
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
            .ok_or_else(|| McpError::ResourceNotFound(uri.to_string()))
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::PROMPTS_LIST, None).await?;

        let prompts: Vec<McpPrompt> = serde_json::from_value(result["prompts"].clone())
            .unwrap_or_default();

        *self.prompts.write().await = prompts.clone();
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

    /// Close the client connection
    pub async fn close(&self) -> Result<(), McpError> {
        let mut transport = self.transport.lock().await;
        transport.close().await?;
        *self.initialized.write().await = false;
        Ok(())
    }

    /// Make a request and wait for response
    async fn call<T>(&self, method: &str, params: Option<Value>) -> Result<T, McpError>
    where
        T: serde::de::DeserializeOwned,
    {
        let id = self.next_request_id();
        let id_str = id.to_string();

        let request = McpRequest::new(id, method);
        let request = if let Some(p) = params {
            request.with_params(p)
        } else {
            request
        };

        // Send request
        {
            let mut transport = self.transport.lock().await;
            transport.send(McpMessage::Request(request)).await?;
        }

        // Wait for response
        let response = self.receive_response(&id_str).await?;

        // Handle response
        match response.into_result() {
            Ok(value) => serde_json::from_value(value).map_err(McpError::from),
            Err(e) => Err(McpError::Server {
                code: e.code,
                message: e.message,
            }),
        }
    }

    /// Send a notification (no response expected)
    async fn notify(&self, method: &str, params: Option<Value>) -> Result<(), McpError> {
        let notification = McpNotification::new(method);
        let notification = if let Some(p) = params {
            notification.with_params(p)
        } else {
            notification
        };

        let mut transport = self.transport.lock().await;
        transport
            .send(McpMessage::Notification(notification))
            .await
    }

    /// Receive a response for the given request ID
    async fn receive_response(&self, expected_id: &str) -> Result<McpResponse, McpError> {
        let mut transport = self.transport.lock().await;

        loop {
            let message = transport.receive().await?;

            match message {
                McpMessage::Response(response) => {
                    if response.id.to_string() == expected_id {
                        return Ok(response);
                    }
                    // Wrong ID, this shouldn't happen in single-threaded operation
                    continue;
                }
                McpMessage::Notification(_) => {
                    // Handle notifications separately if needed
                    continue;
                }
                McpMessage::Request(_) => {
                    // Server shouldn't send requests in typical operation
                    continue;
                }
            }
        }
    }

    /// Generate next request ID
    fn next_request_id(&self) -> RequestId {
        RequestId::Number(self.request_id.fetch_add(1, Ordering::SeqCst) as i64)
    }

    /// Ensure the client is initialized
    async fn ensure_initialized(&self) -> Result<(), McpError> {
        if !*self.initialized.read().await {
            return Err(McpError::NotInitialized);
        }
        Ok(())
    }

    /// Get cached tools
    pub async fn cached_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.clone()
    }

    /// Get cached resources
    pub async fn cached_resources(&self) -> Vec<McpResource> {
        self.resources.read().await.clone()
    }

    /// Get cached prompts
    pub async fn cached_prompts(&self) -> Vec<McpPrompt> {
        self.prompts.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Most tests require a real MCP server
    // These are basic unit tests

    #[test]
    fn test_client_info_default() {
        let info = ClientInfo::default();
        assert_eq!(info.name, "sage-agent");
    }

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo::default(),
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("sage-agent"));
    }
}
