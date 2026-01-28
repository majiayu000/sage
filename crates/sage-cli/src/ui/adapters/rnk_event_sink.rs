//! RnkEventSink - EventSink implementation for rnk UI framework
//!
//! This adapter bridges sage-core's UI abstraction with the rnk framework,
//! enabling the agent to emit events without directly depending on rnk.

use sage_core::ui::bridge::{AgentEvent, EventAdapter};
use sage_core::ui::traits::EventSink;
use std::sync::Arc;

/// EventSink implementation that forwards events to rnk
///
/// This adapter:
/// 1. Forwards events to the EventAdapter for state updates
/// 2. Calls `rnk::request_render()` to trigger UI refresh
pub struct RnkEventSink {
    adapter: Arc<EventAdapter>,
}

impl RnkEventSink {
    /// Create a new RnkEventSink with the given EventAdapter
    pub fn new(adapter: Arc<EventAdapter>) -> Self {
        Self { adapter }
    }

    /// Create a new RnkEventSink with a default EventAdapter
    pub fn with_default_adapter() -> (Self, Arc<EventAdapter>) {
        let adapter = Arc::new(EventAdapter::with_default_state());
        let sink = Self::new(Arc::clone(&adapter));
        (sink, adapter)
    }
}

impl EventSink for RnkEventSink {
    fn handle_event(&self, event: AgentEvent) {
        self.adapter.handle_event(event);
    }

    fn request_refresh(&self) {
        rnk::request_render();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rnk_event_sink_creation() {
        let (sink, adapter) = RnkEventSink::with_default_adapter();
        assert!(Arc::ptr_eq(&sink.adapter, &adapter));
    }

    #[test]
    fn test_rnk_event_sink_handle_event() {
        let (sink, adapter) = RnkEventSink::with_default_adapter();

        sink.handle_event(AgentEvent::ThinkingStarted);

        let state = adapter.get_state();
        assert!(matches!(
            state.phase,
            sage_core::ui::bridge::ExecutionPhase::Thinking
        ));
    }
}
