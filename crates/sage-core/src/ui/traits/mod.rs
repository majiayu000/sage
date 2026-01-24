//! UI Traits - Abstractions for UI framework independence
//!
//! This module provides traits and types that decouple the agent core
//! from specific UI implementations, enabling:
//!
//! - **Testability**: Use mock implementations in tests
//! - **Flexibility**: Support multiple UI frameworks (rnk, ratatui, web, etc.)
//! - **Clean architecture**: No global state, explicit dependency injection
//!
//! # Core Types
//!
//! - [`EventSink`]: Trait for handling agent events
//! - [`UiContext`]: Dependency injection container for UI operations
//! - [`NoopEventSink`]: No-op implementation for testing
//!
//! # Example
//!
//! ```ignore
//! use sage_core::ui::traits::{EventSink, UiContext};
//! use sage_core::ui::AgentEvent;
//! use std::sync::Arc;
//!
//! // Create a custom event sink
//! struct MyEventSink;
//!
//! impl EventSink for MyEventSink {
//!     fn handle_event(&self, event: AgentEvent) {
//!         println!("Event: {:?}", event);
//!     }
//!     fn request_refresh(&self) {}
//! }
//!
//! // Use it with UiContext
//! let ctx = UiContext::new(Arc::new(MyEventSink));
//! ctx.emit(AgentEvent::ThinkingStarted);
//! ```

mod context;
mod event_sink;

pub use context::UiContext;
pub use event_sink::{EventSink, NoopEventSink};

#[cfg(test)]
pub mod testing {
    //! Testing utilities for UI traits

    use super::*;
    use crate::ui::bridge::AgentEvent;
    use std::sync::{Arc, Mutex};

    /// Mock event sink that captures events for testing
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sage_core::ui::traits::testing::MockEventSink;
    /// use sage_core::ui::traits::UiContext;
    /// use sage_core::ui::AgentEvent;
    /// use std::sync::Arc;
    ///
    /// let sink = Arc::new(MockEventSink::new());
    /// let ctx = UiContext::new(sink.clone());
    ///
    /// ctx.emit(AgentEvent::ThinkingStarted);
    ///
    /// assert_eq!(sink.events().len(), 1);
    /// ```
    pub struct MockEventSink {
        events: Arc<Mutex<Vec<AgentEvent>>>,
        refresh_count: Arc<Mutex<u32>>,
    }

    impl MockEventSink {
        /// Create a new mock event sink
        pub fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                refresh_count: Arc::new(Mutex::new(0)),
            }
        }

        /// Get all captured events
        pub fn events(&self) -> Vec<AgentEvent> {
            self.events.lock().unwrap().clone()
        }

        /// Get the number of refresh requests
        pub fn refresh_count(&self) -> u32 {
            *self.refresh_count.lock().unwrap()
        }

        /// Clear all captured events
        pub fn clear(&self) {
            self.events.lock().unwrap().clear();
            *self.refresh_count.lock().unwrap() = 0;
        }
    }

    impl Default for MockEventSink {
        fn default() -> Self {
            Self::new()
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
}
