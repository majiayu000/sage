//! MCP client implementation
//!
//! Provides a high-level client for communicating with MCP servers.
//!
//! # Features
//! - Concurrent request support with proper message routing
//! - Request timeout handling
//! - Notification handling
//! - Background message receiver

use super::error::McpError;
use super::protocol::{
    MCP_PROTOCOL_VERSION, McpMessage, McpNotification, McpRequest, McpResponse, McpRpcError,
    RequestId, methods,
};
use super::transport::McpTransport;
use super::types::{
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, McpCapabilities, McpPrompt,
    McpPromptMessage, McpResource, McpResourceContent, McpServerInfo, McpTool, McpToolResult,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio::time::timeout;
use tracing::{debug, error, instrument, warn};

/// Default request timeout in seconds
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// Message sender command for the background receiver
enum ReceiverCommand {
    /// Register a pending request
    RegisterRequest {
        id: String,
        sender: oneshot::Sender<McpResponse>,
    },
    /// Shutdown the receiver
    Shutdown,
}

/// MCP client for communicating with MCP servers
pub struct McpClient {
    /// Transport layer (for sending)
    transport: Arc<Mutex<Box<dyn McpTransport>>>,
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
    /// Command sender to the background receiver
    command_sender: mpsc::Sender<ReceiverCommand>,
    /// Whether initialized
    initialized: RwLock<bool>,
    /// Whether the client is running
    running: Arc<AtomicBool>,
    /// Request timeout duration
    request_timeout: Duration,
    /// Notification handler
    notification_handler: RwLock<Option<Box<dyn NotificationHandler>>>,
}

/// Trait for handling MCP notifications
pub trait NotificationHandler: Send + Sync {
    /// Handle a notification
    fn handle(&self, method: &str, params: Option<Value>);
}

/// Default notification handler that logs notifications
pub struct LoggingNotificationHandler;

impl NotificationHandler for LoggingNotificationHandler {
    fn handle(&self, method: &str, params: Option<Value>) {
        debug!("MCP notification: {} {:?}", method, params);
    }
}

impl McpClient {
    /// Create a new MCP client with the given transport
    pub fn new(transport: Box<dyn McpTransport>) -> Self {
        Self::with_timeout(transport, Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS))
    }

    /// Create a new MCP client with custom timeout
    pub fn with_timeout(transport: Box<dyn McpTransport>, request_timeout: Duration) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(100);
        let transport = Arc::new(Mutex::new(transport));
        let running = Arc::new(AtomicBool::new(true));

        let client = Self {
            transport: Arc::clone(&transport),
            server_info: RwLock::new(None),
            capabilities: RwLock::new(McpCapabilities::default()),
            tools: RwLock::new(Vec::new()),
            resources: RwLock::new(Vec::new()),
            prompts: RwLock::new(Vec::new()),
            request_id: AtomicU64::new(1),
            command_sender,
            initialized: RwLock::new(false),
            running: Arc::clone(&running),
            request_timeout,
            notification_handler: RwLock::new(Some(Box::new(LoggingNotificationHandler))),
        };

        // Start background message receiver
        let transport_clone = Arc::clone(&transport);
        let running_clone = Arc::clone(&running);
        tokio::spawn(Self::message_receiver(
            transport_clone,
            command_receiver,
            running_clone,
        ));

        client
    }

    /// Background task that receives messages and routes them
    async fn message_receiver(
        transport: Arc<Mutex<Box<dyn McpTransport>>>,
        mut command_receiver: mpsc::Receiver<ReceiverCommand>,
        running: Arc<AtomicBool>,
    ) {
        let mut pending_requests: HashMap<String, oneshot::Sender<McpResponse>> = HashMap::new();

        while running.load(Ordering::SeqCst) {
            tokio::select! {
                // Handle commands from the client
                cmd = command_receiver.recv() => {
                    match cmd {
                        Some(ReceiverCommand::RegisterRequest { id, sender }) => {
                            pending_requests.insert(id, sender);
                        }
                        Some(ReceiverCommand::Shutdown) | None => {
                            debug!("MCP message receiver shutting down");
                            break;
                        }
                    }
                }
                // Receive messages from transport
                result = async {
                    let mut transport = transport.lock().await;
                    transport.receive().await
                } => {
                    match result {
                        Ok(message) => {
                            match message {
                                McpMessage::Response(response) => {
                                    let id = response.id.to_string();
                                    if let Some(sender) = pending_requests.remove(&id) {
                                        if sender.send(response).is_err() {
                                            warn!("Failed to send response to waiting request {}", id);
                                        }
                                    } else {
                                        warn!("Received response for unknown request: {}", id);
                                    }
                                }
                                McpMessage::Notification(notification) => {
                                    debug!("Received notification: {}", notification.method);
                                    // Notifications are logged; custom handlers can be added
                                }
                                McpMessage::Request(request) => {
                                    // Server-initiated requests (rare in current MCP usage)
                                    warn!("Received server request: {}", request.method);
                                }
                            }
                        }
                        Err(e) => {
                            if running.load(Ordering::SeqCst) {
                                error!("Error receiving MCP message: {}", e);
                            }
                            // On connection error, notify all pending requests
                            for (id, sender) in pending_requests.drain() {
                                warn!("Cancelling pending request {} due to connection error", id);
                                let _ = sender.send(McpResponse::error(
                                    RequestId::String(id),
                                    McpRpcError::new(-32000, e.to_string()),
                                ));
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Set a custom notification handler
    pub async fn set_notification_handler(&self, handler: Box<dyn NotificationHandler>) {
        *self.notification_handler.write().await = Some(handler);
    }

    /// Set request timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.request_timeout = timeout;
    }

    /// Initialize the MCP connection
    #[instrument(skip(self), level = "debug")]
    pub async fn initialize(&self) -> Result<McpServerInfo, McpError> {
        if *self.initialized.read().await {
            return Err(McpError::AlreadyInitialized);
        }

        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo::default(),
        };

        let result: InitializeResult = self.call(methods::INITIALIZE, Some(json!(params))).await?;

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
    #[instrument(skip(self), level = "debug")]
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::TOOLS_LIST, None).await?;

        let tools: Vec<McpTool> =
            serde_json::from_value(result["tools"].clone()).unwrap_or_default();

        *self.tools.write().await = tools.clone();
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
            .ok_or_else(|| McpError::resource_not_found(uri.to_string()))
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        self.ensure_initialized().await?;

        let result: Value = self.call(methods::PROMPTS_LIST, None).await?;

        let prompts: Vec<McpPrompt> =
            serde_json::from_value(result["prompts"].clone()).unwrap_or_default();

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
        // Signal the receiver to stop
        self.running.store(false, Ordering::SeqCst);
        let _ = self.command_sender.send(ReceiverCommand::Shutdown).await;

        // Close the transport
        let mut transport = self.transport.lock().await;
        transport.close().await?;
        *self.initialized.write().await = false;
        Ok(())
    }

    /// Make a request and wait for response with timeout
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

        // Create response channel
        let (response_sender, response_receiver) = oneshot::channel();

        // Register the pending request
        self.command_sender
            .send(ReceiverCommand::RegisterRequest {
                id: id_str.clone(),
                sender: response_sender,
            })
            .await
            .map_err(|_| McpError::connection("Failed to register request"))?;

        // Send request
        {
            let mut transport = self.transport.lock().await;
            transport.send(McpMessage::Request(request)).await?;
        }

        // Wait for response with timeout
        let response = timeout(self.request_timeout, response_receiver)
            .await
            .map_err(|_| McpError::timeout(self.request_timeout.as_secs()))?
            .map_err(|_| McpError::connection("Response channel closed"))?;

        // Handle response
        match response.into_result() {
            Ok(value) => serde_json::from_value(value).map_err(McpError::from),
            Err(e) => Err(McpError::server(e.code, e.message)),
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
        transport.send(McpMessage::Notification(notification)).await
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

    /// Refresh all caches (tools, resources, prompts)
    pub async fn refresh_caches(&self) -> Result<(), McpError> {
        self.list_tools().await?;
        self.list_resources().await?;
        self.list_prompts().await?;
        Ok(())
    }

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
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

        let json = serde_json::to_string(&params).expect("Failed to serialize test params");
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("sage-agent"));
    }

    #[test]
    fn test_request_timeout_default() {
        assert_eq!(DEFAULT_REQUEST_TIMEOUT_SECS, 300);
    }
}
