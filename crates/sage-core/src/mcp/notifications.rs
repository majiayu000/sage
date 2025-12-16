//! MCP Notification Handling System
//!
//! Provides a flexible notification handling system for MCP notifications.
//! Supports multiple handlers, filtering, and async processing.

use super::error::McpError;
use super::protocol::McpNotification;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info, warn};

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
    pub fn new(server_name: impl Into<String>, notification: &McpNotification) -> Self {
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

/// Trait for handling notifications
#[async_trait]
pub trait NotificationHandler: Send + Sync {
    /// Handle a notification event
    async fn handle(&self, event: NotificationEvent) -> Result<(), McpError>;

    /// Check if this handler is interested in a particular method
    fn accepts(&self, method: &str) -> bool {
        // Default: accept all methods
        let _ = method;
        true
    }
}

/// A handler that logs notifications
pub struct LoggingHandler {
    /// Log level
    level: LogLevel,
}

/// Log levels for notification logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
}

impl LoggingHandler {
    /// Create a new logging handler
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }
}

#[async_trait]
impl NotificationHandler for LoggingHandler {
    async fn handle(&self, event: NotificationEvent) -> Result<(), McpError> {
        match self.level {
            LogLevel::Debug => debug!(
                "MCP notification from '{}': {} {:?}",
                event.server_name, event.method, event.params
            ),
            LogLevel::Info => info!(
                "MCP notification from '{}': {}",
                event.server_name, event.method
            ),
            LogLevel::Warn => warn!(
                "MCP notification from '{}': {}",
                event.server_name, event.method
            ),
        }
        Ok(())
    }
}

/// A handler that invalidates cache on relevant notifications
pub struct CacheInvalidationHandler {
    /// Reference to the cache
    cache: Arc<super::cache::McpCache>,
}

impl CacheInvalidationHandler {
    /// Create a new cache invalidation handler
    pub fn new(cache: Arc<super::cache::McpCache>) -> Self {
        Self { cache }
    }
}

#[async_trait]
impl NotificationHandler for CacheInvalidationHandler {
    async fn handle(&self, event: NotificationEvent) -> Result<(), McpError> {
        match event.method.as_str() {
            methods::TOOLS_LIST_CHANGED => {
                self.cache.invalidate_tools(&event.server_name);
                debug!(
                    "Invalidated tools cache for '{}' due to notification",
                    event.server_name
                );
            }
            methods::RESOURCES_LIST_CHANGED => {
                self.cache.invalidate_resources(&event.server_name);
                debug!(
                    "Invalidated resources cache for '{}' due to notification",
                    event.server_name
                );
            }
            methods::RESOURCE_UPDATED => {
                if let Some(uri) = event.get_resource_uri() {
                    self.cache.invalidate_resource_content(&uri);
                    debug!(
                        "Invalidated resource content cache for '{}' due to notification",
                        uri
                    );
                }
            }
            methods::PROMPTS_LIST_CHANGED => {
                self.cache.invalidate_prompts(&event.server_name);
                debug!(
                    "Invalidated prompts cache for '{}' due to notification",
                    event.server_name
                );
            }
            _ => {}
        }
        Ok(())
    }

    fn accepts(&self, method: &str) -> bool {
        matches!(
            method,
            methods::TOOLS_LIST_CHANGED
                | methods::RESOURCES_LIST_CHANGED
                | methods::RESOURCE_UPDATED
                | methods::PROMPTS_LIST_CHANGED
        )
    }
}

/// A handler that collects notifications for later retrieval
pub struct CollectorHandler {
    /// Collected events
    events: RwLock<Vec<NotificationEvent>>,
    /// Maximum events to store
    max_events: usize,
}

impl CollectorHandler {
    /// Create a new collector handler
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            max_events,
        }
    }

    /// Get collected events
    pub async fn events(&self) -> Vec<NotificationEvent> {
        self.events.read().await.clone()
    }

    /// Clear collected events
    pub async fn clear(&self) {
        self.events.write().await.clear();
    }

    /// Get events for a specific method
    pub async fn events_for_method(&self, method: &str) -> Vec<NotificationEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| e.method == method)
            .cloned()
            .collect()
    }
}

#[async_trait]
impl NotificationHandler for CollectorHandler {
    async fn handle(&self, event: NotificationEvent) -> Result<(), McpError> {
        let mut events = self.events.write().await;

        // Remove oldest if at capacity
        if events.len() >= self.max_events {
            events.remove(0);
        }

        events.push(event);
        Ok(())
    }
}

/// Function-based notification handler
pub struct FnHandler<F>
where
    F: Fn(NotificationEvent) + Send + Sync,
{
    /// The handler function
    handler: F,
    /// Methods to accept (empty = all)
    methods: Vec<String>,
}

impl<F> FnHandler<F>
where
    F: Fn(NotificationEvent) + Send + Sync,
{
    /// Create a new function handler
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            methods: Vec::new(),
        }
    }

    /// Create a handler for specific methods
    pub fn for_methods(handler: F, methods: Vec<String>) -> Self {
        Self { handler, methods }
    }
}

#[async_trait]
impl<F> NotificationHandler for FnHandler<F>
where
    F: Fn(NotificationEvent) + Send + Sync,
{
    async fn handle(&self, event: NotificationEvent) -> Result<(), McpError> {
        (self.handler)(event);
        Ok(())
    }

    fn accepts(&self, method: &str) -> bool {
        self.methods.is_empty() || self.methods.iter().any(|m| m == method)
    }
}

/// Notification dispatcher that routes notifications to handlers
pub struct NotificationDispatcher {
    /// Registered handlers
    handlers: RwLock<Vec<Arc<dyn NotificationHandler>>>,
    /// Broadcast channel for all notifications
    broadcast_tx: broadcast::Sender<NotificationEvent>,
    /// Whether to process handlers in parallel
    parallel: bool,
}

impl NotificationDispatcher {
    /// Create a new notification dispatcher
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        Self {
            handlers: RwLock::new(Vec::new()),
            broadcast_tx,
            parallel: true,
        }
    }

    /// Create a dispatcher with sequential processing
    pub fn sequential() -> Self {
        let mut dispatcher = Self::new();
        dispatcher.parallel = false;
        dispatcher
    }

    /// Register a handler
    pub async fn register(&self, handler: Arc<dyn NotificationHandler>) {
        self.handlers.write().await.push(handler);
    }

    /// Subscribe to notification broadcasts
    pub fn subscribe(&self) -> broadcast::Receiver<NotificationEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Dispatch a notification to all handlers
    pub async fn dispatch(&self, event: NotificationEvent) {
        // Broadcast to subscribers
        let _ = self.broadcast_tx.send(event.clone());

        // Get handlers that accept this method
        let handlers = self.handlers.read().await;
        let relevant_handlers: Vec<_> = handlers
            .iter()
            .filter(|h| h.accepts(&event.method))
            .cloned()
            .collect();

        if relevant_handlers.is_empty() {
            return;
        }

        if self.parallel {
            // Process handlers in parallel
            let futures: Vec<_> = relevant_handlers
                .into_iter()
                .map(|handler| {
                    let event = event.clone();
                    async move {
                        if let Err(e) = handler.handle(event).await {
                            error!("Notification handler error: {}", e);
                        }
                    }
                })
                .collect();

            futures::future::join_all(futures).await;
        } else {
            // Process handlers sequentially
            for handler in relevant_handlers {
                if let Err(e) = handler.handle(event.clone()).await {
                    error!("Notification handler error: {}", e);
                }
            }
        }
    }

    /// Create a notification event and dispatch it
    pub async fn notify(&self, server_name: &str, notification: &McpNotification) {
        let event = NotificationEvent::new(server_name, notification);
        self.dispatch(event).await;
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating pre-configured dispatchers
pub struct NotificationDispatcherBuilder {
    handlers: Vec<Arc<dyn NotificationHandler>>,
    parallel: bool,
}

impl NotificationDispatcherBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            parallel: true,
        }
    }

    /// Add a handler
    pub fn with_handler(mut self, handler: Arc<dyn NotificationHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Add logging handler
    pub fn with_logging(self, level: LogLevel) -> Self {
        self.with_handler(Arc::new(LoggingHandler::new(level)))
    }

    /// Add cache invalidation handler
    pub fn with_cache_invalidation(self, cache: Arc<super::cache::McpCache>) -> Self {
        self.with_handler(Arc::new(CacheInvalidationHandler::new(cache)))
    }

    /// Add collector handler
    pub fn with_collector(self, max_events: usize) -> Self {
        self.with_handler(Arc::new(CollectorHandler::new(max_events)))
    }

    /// Set sequential processing
    pub fn sequential(mut self) -> Self {
        self.parallel = false;
        self
    }

    /// Build the dispatcher
    pub async fn build(self) -> NotificationDispatcher {
        let dispatcher = if self.parallel {
            NotificationDispatcher::new()
        } else {
            NotificationDispatcher::sequential()
        };

        for handler in self.handlers {
            dispatcher.register(handler).await;
        }

        dispatcher
    }
}

impl Default for NotificationDispatcherBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_event_creation() {
        let notification = McpNotification::new(methods::TOOLS_LIST_CHANGED);
        let event = NotificationEvent::new("server1", &notification);

        assert_eq!(event.server_name, "server1");
        assert_eq!(event.method, methods::TOOLS_LIST_CHANGED);
        assert!(event.is_tools_changed());
    }

    #[test]
    fn test_notification_event_resource_uri() {
        let mut notification = McpNotification::new(methods::RESOURCE_UPDATED);
        notification.params = Some(serde_json::json!({
            "uri": "file:///tmp/test.txt"
        }));

        let event = NotificationEvent::new("server1", &notification);

        assert!(event.is_resource_updated());
        assert_eq!(
            event.get_resource_uri(),
            Some("file:///tmp/test.txt".to_string())
        );
    }

    #[test]
    fn test_progress_info() {
        let mut notification = McpNotification::new(methods::PROGRESS);
        notification.params = Some(serde_json::json!({
            "progressToken": "task-1",
            "progress": 50.0,
            "total": 100.0
        }));

        let event = NotificationEvent::new("server1", &notification);
        let progress = event.get_progress().unwrap();

        assert_eq!(progress.token, "task-1");
        assert_eq!(progress.progress, 50.0);
        assert_eq!(progress.total, Some(100.0));
        assert_eq!(progress.percentage(), Some(50.0));
    }

    #[tokio::test]
    async fn test_logging_handler() {
        let handler = LoggingHandler::new(LogLevel::Debug);
        let notification = McpNotification::new("test/method");
        let event = NotificationEvent::new("server1", &notification);

        let result = handler.handle(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_collector_handler() {
        let handler = CollectorHandler::new(10);
        let notification = McpNotification::new("test/method");
        let event = NotificationEvent::new("server1", &notification);

        handler.handle(event).await.unwrap();

        let events = handler.events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].method, "test/method");
    }

    #[tokio::test]
    async fn test_collector_max_events() {
        let handler = CollectorHandler::new(2);

        for i in 0..3 {
            let notification = McpNotification::new(format!("test/{}", i));
            let event = NotificationEvent::new("server1", &notification);
            handler.handle(event).await.unwrap();
        }

        let events = handler.events().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].method, "test/1");
        assert_eq!(events[1].method, "test/2");
    }

    #[tokio::test]
    async fn test_dispatcher() {
        let dispatcher = NotificationDispatcher::new();
        let collector = Arc::new(CollectorHandler::new(10));

        dispatcher.register(collector.clone()).await;

        let notification = McpNotification::new("test/method");
        dispatcher.notify("server1", &notification).await;

        let events = collector.events().await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_dispatcher_broadcast() {
        let dispatcher = NotificationDispatcher::new();
        let mut receiver = dispatcher.subscribe();

        let notification = McpNotification::new("test/method");
        dispatcher.notify("server1", &notification).await;

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.method, "test/method");
    }

    #[tokio::test]
    async fn test_fn_handler() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handler = FnHandler::new(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let notification = McpNotification::new("test/method");
        let event = NotificationEvent::new("server1", &notification);

        handler.handle(event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_dispatcher_builder() {
        let dispatcher = NotificationDispatcherBuilder::new()
            .with_logging(LogLevel::Debug)
            .with_collector(10)
            .build()
            .await;

        let notification = McpNotification::new("test/method");
        dispatcher.notify("server1", &notification).await;

        // Verify it doesn't panic
    }
}
