//! MCP Notification Handling System
//!
//! Provides a flexible notification handling system for MCP notifications.
//! Supports multiple handlers, filtering, and async processing.

mod handlers;
mod processor;
mod types;

#[cfg(test)]
mod tests;

// Re-export all public items
pub use handlers::{
    CacheInvalidationHandler, CollectorHandler, FnHandler, LoggingHandler, NotificationHandler,
};
pub use processor::{NotificationDispatcher, NotificationDispatcherBuilder};
pub use types::{LogLevel, NotificationEvent, ProgressInfo, methods};
