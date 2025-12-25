//! Notification processing and dispatching

use super::handlers::{CollectorHandler, LoggingHandler, NotificationHandler, CacheInvalidationHandler};
use super::types::{LogLevel, NotificationEvent};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::error;

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
    pub async fn notify(&self, server_name: &str, notification: &crate::mcp::protocol::McpNotification) {
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
    pub fn with_cache_invalidation(self, cache: Arc<crate::mcp::cache::McpCache>) -> Self {
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
