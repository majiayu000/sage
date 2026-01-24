//! UiContext - Dependency injection container for UI operations
//!
//! This module provides a context object that can be passed through
//! the agent execution stack, replacing global state with explicit
//! dependency injection.

use super::event_sink::{EventSink, NoopEventSink};
use crate::ui::bridge::AgentEvent;
use std::sync::Arc;

/// UI context for dependency injection
///
/// This struct holds references to UI-related services and can be
/// passed through the agent execution stack. It replaces the need
/// for global singletons like `emit_event`.
///
/// # Example
///
/// ```ignore
/// use sage_core::ui::traits::{UiContext, EventSink};
/// use sage_core::ui::AgentEvent;
/// use std::sync::Arc;
///
/// // Create a context with a custom event sink
/// let sink = Arc::new(MyEventSink::new());
/// let ctx = UiContext::new(sink);
///
/// // Emit events through the context
/// ctx.emit(AgentEvent::ThinkingStarted);
/// ```
#[derive(Clone)]
pub struct UiContext {
    event_sink: Arc<dyn EventSink>,
}

impl UiContext {
    /// Create a new UI context with the given event sink
    pub fn new(event_sink: Arc<dyn EventSink>) -> Self {
        Self { event_sink }
    }

    /// Create a no-op UI context (for testing or when UI is disabled)
    pub fn noop() -> Self {
        Self {
            event_sink: Arc::new(NoopEventSink),
        }
    }

    /// Emit an event to the UI
    ///
    /// This handles the event and requests a UI refresh.
    pub fn emit(&self, event: AgentEvent) {
        self.event_sink.handle_event(event);
        self.event_sink.request_refresh();
    }

    /// Emit an event without requesting a refresh
    ///
    /// Use this when batching multiple events and you want to
    /// refresh only once at the end.
    pub fn emit_no_refresh(&self, event: AgentEvent) {
        self.event_sink.handle_event(event);
    }

    /// Request a UI refresh
    ///
    /// Call this after batching multiple events with `emit_no_refresh`.
    pub fn refresh(&self) {
        self.event_sink.request_refresh();
    }

    /// Get a reference to the underlying event sink
    pub fn event_sink(&self) -> &Arc<dyn EventSink> {
        &self.event_sink
    }
}

impl Default for UiContext {
    fn default() -> Self {
        Self::noop()
    }
}

impl std::fmt::Debug for UiContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiContext")
            .field("event_sink", &"<dyn EventSink>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct TestEventSink {
        events: Arc<Mutex<Vec<AgentEvent>>>,
        refresh_count: Arc<Mutex<u32>>,
    }

    impl TestEventSink {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                refresh_count: Arc::new(Mutex::new(0)),
            }
        }

        fn events(&self) -> Vec<AgentEvent> {
            self.events.lock().unwrap().clone()
        }

        fn refresh_count(&self) -> u32 {
            *self.refresh_count.lock().unwrap()
        }
    }

    impl EventSink for TestEventSink {
        fn handle_event(&self, event: AgentEvent) {
            self.events.lock().unwrap().push(event);
        }

        fn request_refresh(&self) {
            *self.refresh_count.lock().unwrap() += 1;
        }
    }

    #[test]
    fn test_ui_context_emit() {
        let sink = Arc::new(TestEventSink::new());
        let ctx = UiContext::new(sink.clone());

        ctx.emit(AgentEvent::ThinkingStarted);

        assert_eq!(sink.events().len(), 1);
        assert_eq!(sink.refresh_count(), 1);
    }

    #[test]
    fn test_ui_context_emit_no_refresh() {
        let sink = Arc::new(TestEventSink::new());
        let ctx = UiContext::new(sink.clone());

        ctx.emit_no_refresh(AgentEvent::ThinkingStarted);
        ctx.emit_no_refresh(AgentEvent::ThinkingStopped);

        assert_eq!(sink.events().len(), 2);
        assert_eq!(sink.refresh_count(), 0);

        ctx.refresh();
        assert_eq!(sink.refresh_count(), 1);
    }

    #[test]
    fn test_ui_context_noop() {
        let ctx = UiContext::noop();
        ctx.emit(AgentEvent::ThinkingStarted);
    }

    #[test]
    fn test_ui_context_default() {
        let ctx = UiContext::default();
        ctx.emit(AgentEvent::ThinkingStarted);
    }

    #[test]
    fn test_ui_context_clone() {
        let sink = Arc::new(TestEventSink::new());
        let ctx1 = UiContext::new(sink.clone());
        let ctx2 = ctx1.clone();

        ctx1.emit(AgentEvent::ThinkingStarted);
        ctx2.emit(AgentEvent::ThinkingStopped);

        assert_eq!(sink.events().len(), 2);
    }
}
