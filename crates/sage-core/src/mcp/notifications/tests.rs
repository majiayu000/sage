//! Tests for notification system

#[cfg(test)]
mod tests {
    use crate::mcp::notifications::handlers::{CollectorHandler, FnHandler, LoggingHandler, NotificationHandler};
    use crate::mcp::notifications::processor::{NotificationDispatcher, NotificationDispatcherBuilder};
    use crate::mcp::notifications::types::{LogLevel, NotificationEvent, methods};
    use crate::mcp::protocol::McpNotification;
    use std::sync::Arc;

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
