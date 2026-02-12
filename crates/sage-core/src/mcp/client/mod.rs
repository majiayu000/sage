//! MCP client implementation
//!
//! Provides a high-level client for communicating with MCP servers.
//!
//! # Features
//! - Concurrent request support with proper message routing
//! - Request timeout handling
//! - Notification handling
//! - Background message receiver

mod notification;
mod operations;
mod receiver;

pub use notification::{LoggingNotificationHandler, SyncNotificationHandler};

use super::error::McpError;
use super::protocol::{McpMessage, McpRequest, RequestId, methods};
use super::transport::McpTransport;
use super::types::{
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, McpCapabilities,
    McpPrompt, McpResource, McpServerInfo, McpTool,
};
use receiver::ReceiverCommand;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::instrument;

/// Default request timeout in seconds
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 300; // 5 minutes

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
    notification_handler: RwLock<Option<Box<dyn SyncNotificationHandler>>>,
    /// Background message receiver task handle
    receiver_handle: StdMutex<Option<JoinHandle<()>>>,
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

        // Start background message receiver
        let transport_clone = Arc::clone(&transport);
        let running_clone = Arc::clone(&running);
        let receiver_handle = tokio::spawn(receiver::message_receiver(
            transport_clone,
            command_receiver,
            running_clone,
        ));

        Self {
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
            receiver_handle: StdMutex::new(Some(receiver_handle)),
        }
    }

    /// Set a custom notification handler
    pub async fn set_notification_handler(&self, handler: Box<dyn SyncNotificationHandler>) {
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
            protocol_version: super::protocol::MCP_PROTOCOL_VERSION.to_string(),
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

    /// Close the client connection
    pub async fn close(&self) -> Result<(), McpError> {
        // Signal the receiver to stop
        self.running.store(false, Ordering::SeqCst);
        let _ = self.command_sender.send(ReceiverCommand::Shutdown).await;

        // Close the transport
        let mut transport = self.transport.lock().await;
        transport.close().await?;

        // Wait for background receiver to finish
        let handle = {
            let mut guard = self
                .receiver_handle
                .lock()
                .map_err(|_| McpError::other("receiver handle lock poisoned"))?;
            guard.take()
        };
        if let Some(handle) = handle {
            let _ = handle.await;
        }

        *self.initialized.write().await = false;
        Ok(())
    }

    /// Make a request and wait for response with timeout
    pub(crate) async fn call<T>(&self, method: &str, params: Option<Value>) -> Result<T, McpError>
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
        let notification = super::protocol::McpNotification::new(method);
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
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        match i64::try_from(id) {
            Ok(n) => RequestId::Number(n),
            Err(_) => RequestId::String(format!("req-{}", id)),
        }
    }

    /// Ensure the client is initialized
    pub(crate) async fn ensure_initialized(&self) -> Result<(), McpError> {
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

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub(crate) fn tools(&self) -> &RwLock<Vec<McpTool>> {
        &self.tools
    }

    pub(crate) fn resources(&self) -> &RwLock<Vec<McpResource>> {
        &self.resources
    }

    pub(crate) fn prompts(&self) -> &RwLock<Vec<McpPrompt>> {
        &self.prompts
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.receiver_handle.lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_info_default() {
        let info = ClientInfo::default();
        assert_eq!(info.name, "sage-agent");
    }

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams {
            protocol_version: super::super::protocol::MCP_PROTOCOL_VERSION.to_string(),
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
