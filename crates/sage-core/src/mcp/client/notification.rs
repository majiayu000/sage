//! Notification handler traits and implementations

use serde_json::Value;
use tracing::debug;

/// Sync trait for handling MCP notifications in the client message loop.
///
/// This is a lightweight synchronous handler used internally by `McpClient`.
/// For the full async notification handling system with filtering and dispatching,
/// see [`super::super::notifications::NotificationHandler`].
pub trait SyncNotificationHandler: Send + Sync {
    /// Handle a notification
    fn handle(&self, method: &str, params: Option<Value>);
}

/// Default notification handler that logs notifications
pub struct LoggingNotificationHandler;

impl SyncNotificationHandler for LoggingNotificationHandler {
    fn handle(&self, method: &str, params: Option<Value>) {
        debug!("MCP notification: {} {:?}", method, params);
    }
}
