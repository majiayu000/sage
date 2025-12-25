//! Notification types and event structures

use serde_json::Value;

/// Standard MCP notification methods
pub mod methods {
    /// Tools list changed notification
    pub const TOOLS_LIST_CHANGED: &str = "notifications/tools/listChanged";
    /// Resources list changed notification
    pub const RESOURCES_LIST_CHANGED: &str = "notifications/resources/listChanged";
    /// Resource updated notification
    pub const RESOURCE_UPDATED: &str = "notifications/resources/updated";
    /// Prompts list changed notification
    pub const PROMPTS_LIST_CHANGED: &str = "notifications/prompts/listChanged";
    /// Progress notification
    pub const PROGRESS: &str = "notifications/progress";
    /// Log message notification
    pub const LOG_MESSAGE: &str = "notifications/message";
    /// Cancelled notification
    pub const CANCELLED: &str = "notifications/cancelled";
}

/// A notification event received from an MCP server
#[derive(Debug, Clone)]
pub struct NotificationEvent {
    /// The server name that sent the notification
    pub server_name: String,
    /// The notification method
    pub method: String,
    /// The notification parameters
    pub params: Option<Value>,
    /// Timestamp when the notification was received
    pub timestamp: std::time::Instant,
}

impl NotificationEvent {
    /// Create a new notification event
    pub fn new(
        server_name: impl Into<String>,
        notification: &super::super::protocol::McpNotification,
    ) -> Self {
        Self {
            server_name: server_name.into(),
            method: notification.method.clone(),
            params: notification.params.clone(),
            timestamp: std::time::Instant::now(),
        }
    }

    /// Check if this is a tools list changed notification
    pub fn is_tools_changed(&self) -> bool {
        self.method == methods::TOOLS_LIST_CHANGED
    }

    /// Check if this is a resources list changed notification
    pub fn is_resources_changed(&self) -> bool {
        self.method == methods::RESOURCES_LIST_CHANGED
    }

    /// Check if this is a resource updated notification
    pub fn is_resource_updated(&self) -> bool {
        self.method == methods::RESOURCE_UPDATED
    }

    /// Check if this is a prompts list changed notification
    pub fn is_prompts_changed(&self) -> bool {
        self.method == methods::PROMPTS_LIST_CHANGED
    }

    /// Check if this is a progress notification
    pub fn is_progress(&self) -> bool {
        self.method == methods::PROGRESS
    }

    /// Get the URI from a resource updated notification
    pub fn get_resource_uri(&self) -> Option<String> {
        self.params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// Get progress information
    pub fn get_progress(&self) -> Option<ProgressInfo> {
        self.params.as_ref().and_then(|p| {
            let progress_token = p
                .get("progressToken")
                .and_then(|v| v.as_str())
                .map(String::from)?;
            let progress = p.get("progress").and_then(|v| v.as_f64())?;
            let total = p.get("total").and_then(|v| v.as_f64());

            Some(ProgressInfo {
                token: progress_token,
                progress,
                total,
            })
        })
    }
}

/// Progress information from a notification
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Progress token
    pub token: String,
    /// Current progress value
    pub progress: f64,
    /// Total value (if known)
    pub total: Option<f64>,
}

impl ProgressInfo {
    /// Get percentage if total is known
    pub fn percentage(&self) -> Option<f64> {
        self.total.map(|t| (self.progress / t) * 100.0)
    }
}

/// Log levels for notification logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
}
