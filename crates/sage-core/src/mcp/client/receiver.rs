//! Background message receiver for MCP client

use super::super::protocol::{McpMessage, McpResponse, McpRpcError, RequestId};
use super::super::transport::McpTransport;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, mpsc, oneshot};
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
