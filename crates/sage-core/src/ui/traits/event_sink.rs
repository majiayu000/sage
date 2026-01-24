//! EventSink trait - Abstraction for UI event handling
//!
//! This trait replaces the global `emit_event` function, enabling:
//! - Dependency injection for testability
//! - Multiple UI framework support
//! - No global state

use crate::ui::bridge::AgentEvent;

/// Trait for handling agent events and triggering UI updates
///
/// Implementations of this trait receive events from the agent execution
/// and are responsible for updating the UI accordingly.
///
/// # Example
///
/// ```ignore
/// use sage_core::ui::traits::EventSink;
/// use sage_core::ui::AgentEvent;
///
/// struct MyEventSink;
///
/// impl EventSink for MyEventSink {
///     fn handle_event(&self, event: AgentEvent) {
///         println!("Received event: {:?}", event);
///     }
///
///     fn request_refresh(&self) {
///         // Trigger UI redraw
///     }
/// }
/// ```
pub trait EventSink: Send + Sync {
    /// Handle an agent event
    ///
    /// This method is called whenever the agent emits an event that
    /// should be reflected in the UI.
    fn handle_event(&self, event: AgentEvent);

    /// Request a UI refresh/redraw
    ///
    /// Called after state updates to trigger the UI framework to re-render.
    /// Some frameworks may batch these requests for efficiency.
    fn request_refresh(&self);
}

/// A no-op event sink for testing or when UI is disabled
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn handle_event(&self, _event: AgentEvent) {}
    fn request_refresh(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock event sink that captures events for testing
    pub struct MockEventSink {
        events: Arc<Mutex<Vec<AgentEvent>>>,
        refresh_count: Arc<Mutex<u32>>,
    }

    impl MockEventSink {
        pub fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                refresh_count: Arc::new(Mutex::new(0)),
            }
        }

        pub fn events(&self) -> Vec<AgentEvent> {
            self.events.lock().unwrap().clone()
        }

        pub fn refresh_count(&self) -> u32 {
            *self.refresh_count.lock().unwrap()
        }
    }

    impl EventSink for MockEventSink {
        fn handle_event(&self, event: AgentEvent) {
            self.events.lock().unwrap().push(event);
        }

        fn request_refresh(&self) {
            *self.refresh_count.lock().unwrap() += 1;
        }
    }

    #[test]
    fn test_noop_event_sink() {
        let sink = NoopEventSink;
        sink.handle_event(AgentEvent::ThinkingStarted);
        sink.request_refresh();
    }

    #[test]
    fn test_mock_event_sink() {
        let sink = MockEventSink::new();

        sink.handle_event(AgentEvent::ThinkingStarted);
        sink.handle_event(AgentEvent::ThinkingStopped);
        sink.request_refresh();

        assert_eq!(sink.events().len(), 2);
        assert_eq!(sink.refresh_count(), 1);
    }
}
