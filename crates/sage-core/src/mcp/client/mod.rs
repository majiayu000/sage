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
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, McpCapabilities, McpPrompt,
    McpResource, McpServerInfo, McpTool,
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
    /// Cached tools (shared with the background receiver for listChanged revoke)
    tools: Arc<RwLock<Vec<McpTool>>>,
    /// When true, `call_tool` only allows names present in the trusted-tool cache.
    /// Registry refresh activates this; direct clients leave it off so
    /// initialize → list_tools → call_tool keeps working.
    /// Shared with the receiver so `notifications/tools/listChanged` can clear
    /// the allowlist without waiting for the next registry refresh.
    trusted_tool_allowlist: Arc<AtomicBool>,
    /// Bumped whenever the trusted-tool cache is cleared or replaced so
    /// `call_tool` and capability refresh can detect invalidation without
    /// holding the tools `RwLock` across remote awaits (which deadlocks the
    /// receiver when listChanged needs the write lock).
    list_changed_generation: Arc<AtomicU64>,
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
        let tools = Arc::new(RwLock::new(Vec::new()));
        let trusted_tool_allowlist = Arc::new(AtomicBool::new(false));
        let list_changed_generation = Arc::new(AtomicU64::new(0));

        // Start background message receiver
        let transport_clone = Arc::clone(&transport);
        let running_clone = Arc::clone(&running);
        let receiver_handle = tokio::spawn(receiver::message_receiver(
            transport_clone,
            command_receiver,
            running_clone,
            Arc::clone(&tools),
            Arc::clone(&trusted_tool_allowlist),
            Arc::clone(&list_changed_generation),
        ));

        Self {
            transport: Arc::clone(&transport),
            server_info: RwLock::new(None),
            capabilities: RwLock::new(McpCapabilities::default()),
            tools,
            trusted_tool_allowlist,
            list_changed_generation,
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
        self.apply_initialize_result(&result).await;

        // Send initialized notification
        self.notify(methods::INITIALIZED, None).await?;

        Ok(result.server_info)
    }

    async fn set_server_info(&self, info: McpServerInfo) {
        let mut server_info = self.server_info.write().await;
        *server_info = Some(info);
    }

    async fn set_capabilities(&self, capabilities: McpCapabilities) {
        let mut capabilities_guard = self.capabilities.write().await;
        *capabilities_guard = capabilities;
    }

    async fn set_initialized(&self, initialized: bool) {
        let mut initialized_guard = self.initialized.write().await;
        *initialized_guard = initialized;
    }

    async fn apply_initialize_result(&self, result: &InitializeResult) {
        self.set_server_info(result.server_info.clone()).await;
        self.set_capabilities(result.capabilities.clone()).await;
        self.set_initialized(true).await;
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

    fn take_receiver_handle(&self) -> Result<Option<JoinHandle<()>>, McpError> {
        let mut guard = self
            .receiver_handle
            .lock()
            .map_err(|_| McpError::other("receiver handle lock poisoned"))?;
        Ok(guard.take())
    }

    /// Close the client connection
    pub async fn close(&self) -> Result<(), McpError> {
        // Signal the receiver to stop
        self.running.store(false, Ordering::SeqCst);
        if let Err(e) = self.command_sender.send(ReceiverCommand::Shutdown).await {
            tracing::debug!("Failed to send MCP shutdown command: {}", e);
        }

        // Close the transport
        {
            let mut transport = self.transport.lock().await;
            transport.close().await?;
        }

        // Wait for background receiver to finish
        if let Some(handle) = self.take_receiver_handle()? {
            if let Err(e) = handle.await {
                tracing::debug!("MCP receiver task ended with error: {}", e);
            }
        }

        self.set_initialized(false).await;
        Ok(())
    }

    /// Make a request and wait for response with timeout
    pub(crate) async fn call<T>(&self, method: &str, params: Option<Value>) -> Result<T, McpError>
    where
        T: serde::de::DeserializeOwned,
    {
        let response_receiver = self.begin_call(method, params).await?;
        self.finish_call(response_receiver).await
    }

    /// Register and send a request without waiting for the response.
    ///
    /// Prefer [`Self::begin_authorized_tool_call`] when the tools write lock must
    /// stay held across dispatch; that path reserves the command-queue slot
    /// before taking the lock so a full bounded channel cannot deadlock against
    /// the receiver's listChanged clear.
    pub(crate) async fn begin_call(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<oneshot::Receiver<super::protocol::McpResponse>, McpError> {
        let id = self.next_request_id();
        let id_str = id.to_string();

        let request = McpRequest::new(id, method);
        let request = if let Some(p) = params {
            request.with_params(p)
        } else {
            request
        };

        let (response_sender, response_receiver) = oneshot::channel();

        self.command_sender
            .send(ReceiverCommand::RegisterRequest {
                id: id_str,
                sender: response_sender,
            })
            .await
            .map_err(|_| McpError::connection("Failed to register request"))?;

        {
            let mut transport = self.transport.lock().await;
            transport.send(McpMessage::Request(request)).await?;
        }

        Ok(response_receiver)
    }

    /// Authorize a trusted tool and dispatch under transport → tools locks.
    ///
    /// Reserves a command-queue permit before acquiring locks so backpressure
    /// cannot deadlock against listChanged. Lock order matches the receiver's
    /// listChanged path (transport then tools write) so revoke-while-receive
    /// cannot race a stale authorized send.
    pub(super) async fn begin_authorized_tool_call(
        &self,
        name: &str,
        params: Value,
    ) -> Result<oneshot::Receiver<super::protocol::McpResponse>, McpError> {
        let permit = self
            .command_sender
            .reserve()
            .await
            .map_err(|_| McpError::connection("Failed to reserve request registration"))?;

        let mut transport = self.transport.lock().await;
        let authorization = self.tools.write().await;
        if !authorization.iter().any(|tool| tool.name == name) {
            return Err(McpError::tool_not_found(name.to_string()));
        }

        let id = self.next_request_id();
        let id_str = id.to_string();
        let request = McpRequest::new(id, methods::TOOLS_CALL).with_params(params);
        let (response_sender, response_receiver) = oneshot::channel();
        permit.send(ReceiverCommand::RegisterRequest {
            id: id_str,
            sender: response_sender,
        });
        transport.send(McpMessage::Request(request)).await?;
        drop(authorization);
        drop(transport);
        Ok(response_receiver)
    }

    pub(crate) async fn finish_call<T>(
        &self,
        response_receiver: oneshot::Receiver<super::protocol::McpResponse>,
    ) -> Result<T, McpError>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = timeout(self.request_timeout, response_receiver)
            .await
            .map_err(|_| McpError::timeout(self.request_timeout.as_secs()))?
            .map_err(|_| McpError::connection("Response channel closed"))?;

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

    /// Whether registry trust filtering has activated the call_tool allowlist.
    pub(crate) fn trusted_tool_allowlist_active(&self) -> bool {
        self.trusted_tool_allowlist.load(Ordering::Acquire)
    }

    /// Generation bumped on listChanged clears and trusted-tool replacements.
    pub(crate) fn list_changed_generation(&self) -> u64 {
        self.list_changed_generation.load(Ordering::Acquire)
    }

    /// Replace the trusted-tool cache and require call_tool to consult it.
    pub(crate) async fn replace_trusted_tools(&self, tools: Vec<McpTool>) {
        let mut guard = self.tools.write().await;
        let allowlist_was_active = self.trusted_tool_allowlist.load(Ordering::Acquire);
        let auth_changed =
            !allowlist_was_active || trusted_tool_names_differ(guard.as_slice(), tools.as_slice());
        *guard = tools;
        self.trusted_tool_allowlist.store(true, Ordering::Release);
        if auth_changed {
            self.list_changed_generation.fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Publish trusted tools only when `expected_generation` still matches.
    ///
    /// Holds the tools write lock across the generation check and cache write so
    /// a concurrent listChanged clear cannot be overwritten by a stale refresh.
    /// Advances the generation only when the authorized name set changes (or the
    /// allowlist is first activated), so identical republishes do not invalidate
    /// in-flight calls.
    pub(crate) async fn replace_trusted_tools_if_generation(
        &self,
        tools: Vec<McpTool>,
        expected_generation: u64,
    ) -> Result<(), McpError> {
        let mut guard = self.tools.write().await;
        if self.list_changed_generation.load(Ordering::Acquire) != expected_generation {
            return Err(McpError::schema(
                "MCP tools/listChanged during trusted-tool publish; retry required".to_string(),
            ));
        }
        let allowlist_was_active = self.trusted_tool_allowlist.load(Ordering::Acquire);
        let auth_changed =
            !allowlist_was_active || trusted_tool_names_differ(guard.as_slice(), tools.as_slice());
        *guard = tools;
        self.trusted_tool_allowlist.store(true, Ordering::Release);
        if auth_changed {
            self.list_changed_generation.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
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

    pub(crate) fn resources(&self) -> &RwLock<Vec<McpResource>> {
        &self.resources
    }

    pub(crate) fn tools(&self) -> &RwLock<Vec<McpTool>> {
        self.tools.as_ref()
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

fn trusted_tool_names_differ(previous: &[McpTool], next: &[McpTool]) -> bool {
    if previous.len() != next.len() {
        return true;
    }
    let mut previous_names: Vec<&str> = previous.iter().map(|tool| tool.name.as_str()).collect();
    let mut next_names: Vec<&str> = next.iter().map(|tool| tool.name.as_str()).collect();
    previous_names.sort_unstable();
    next_names.sort_unstable();
    previous_names != next_names
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
