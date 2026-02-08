//! Built-in notification handlers

use super::types::{LogLevel, NotificationEvent, methods};
use crate::mcp::error::McpError;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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
    cache: Arc<crate::mcp::cache::McpCache>,
}

impl CacheInvalidationHandler {
    /// Create a new cache invalidation handler
    pub fn new(cache: Arc<crate::mcp::cache::McpCache>) -> Self {
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
    events: RwLock<VecDeque<NotificationEvent>>,
    /// Maximum events to store
    max_events: usize,
}

impl CollectorHandler {
    /// Create a new collector handler
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::new()),
            max_events,
        }
    }

    /// Get collected events
    pub async fn events(&self) -> Vec<NotificationEvent> {
        self.events.read().await.iter().cloned().collect()
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
            events.pop_front();
        }

        events.push_back(event);
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
