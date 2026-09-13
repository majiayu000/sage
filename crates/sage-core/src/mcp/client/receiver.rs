//! Background message receiver for MCP client

use super::super::notifications::methods as notification_methods;
use super::super::protocol::{McpMessage, McpResponse, McpRpcError, RequestId};
use super::super::transport::McpTransport;
use super::super::types::McpTool;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tracing::{debug, error, warn};

/// Message sender command for the background receiver
pub(super) enum ReceiverCommand {
    /// Register a pending request
    RegisterRequest {
        id: String,
        sender: oneshot::Sender<McpResponse>,
    },
    /// Shutdown the receiver
    Shutdown,
}

/// Background task that receives messages and routes them
pub(super) async fn message_receiver(
    transport: Arc<Mutex<Box<dyn McpTransport>>>,
    mut command_receiver: mpsc::Receiver<ReceiverCommand>,
    running: Arc<AtomicBool>,
    tools: Arc<RwLock<Vec<McpTool>>>,
    trusted_tool_allowlist: Arc<AtomicBool>,
    list_changed_generation: Arc<AtomicU64>,
) {
    let mut pending_requests: HashMap<String, oneshot::Sender<McpResponse>> = HashMap::new();

    while running.load(Ordering::SeqCst) {
        // Prefer a ready transport receive (including listChanged revoke) over
        // RegisterRequest when both are ready. An unbiased select can cancel the
        // receive future, release the transport mutex, and let call_tool send
        // under a still-authorized allowlist before the notification is drained.
        tokio::select! {
            biased;
            // Receive messages from transport. For listChanged, revoke the
            // allowlist before releasing the transport mutex so a concurrent
            // call_tool cannot send under stale authorization between receive
            // and clear (call_tool acquires transport before the tools lock).
            result = {
                let transport = Arc::clone(&transport);
                let tools = Arc::clone(&tools);
                let trusted_tool_allowlist = Arc::clone(&trusted_tool_allowlist);
                let list_changed_generation = Arc::clone(&list_changed_generation);
                async move {
                    let mut transport = transport.lock().await;
                    let received = transport.receive().await;
                    if let Ok(McpMessage::Notification(ref notification)) = received {
                        if notification.method == notification_methods::TOOLS_LIST_CHANGED {
                            clear_trusted_allowlist_if_active(
                                &tools,
                                &trusted_tool_allowlist,
                                &list_changed_generation,
                            )
                            .await;
                        }
                    }
                    received
                }
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
            // Handle commands from the client only when receive is not ready.
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
        }
    }
}

/// Fail-closed: clear the registry allowlist so drifted/changed tools cannot be
/// called by name until the next capability refresh.
pub(super) async fn clear_trusted_allowlist_if_active(
    tools: &RwLock<Vec<McpTool>>,
    trusted_tool_allowlist: &AtomicBool,
    list_changed_generation: &AtomicU64,
) {
    if trusted_tool_allowlist.load(Ordering::Acquire) {
        tools.write().await.clear();
        list_changed_generation.fetch_add(1, Ordering::AcqRel);
        debug!("Cleared trusted-tool allowlist after tools/listChanged");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::types::McpTool;

    #[tokio::test]
    async fn list_changed_clears_active_allowlist() {
        let tools = RwLock::new(vec![McpTool::new("read")]);
        let allowlist = AtomicBool::new(true);
        let generation = AtomicU64::new(0);
        clear_trusted_allowlist_if_active(&tools, &allowlist, &generation).await;
        assert!(tools.read().await.is_empty());
        assert!(allowlist.load(Ordering::Acquire));
        assert_eq!(generation.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn list_changed_leaves_inactive_direct_client_cache() {
        let tools = RwLock::new(vec![McpTool::new("read")]);
        let allowlist = AtomicBool::new(false);
        let generation = AtomicU64::new(0);
        clear_trusted_allowlist_if_active(&tools, &allowlist, &generation).await;
        assert_eq!(tools.read().await.len(), 1);
        assert_eq!(generation.load(Ordering::Acquire), 0);
    }
}
