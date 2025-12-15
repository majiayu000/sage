//! Event system for async code agent operations
//!
//! This module provides a broadcast-based event bus for system-wide event distribution.
//! It enables loose coupling between components by allowing them to publish and subscribe
//! to events without direct dependencies.

use std::sync::Arc;
use tokio::sync::broadcast;

/// System-wide events that can be published through the EventBus
#[derive(Debug, Clone)]
pub enum Event {
    // ========== Stream Events ==========
    /// LLM stream connected
    StreamConnected {
        provider: String,
        model: String,
    },

    /// LLM stream disconnected
    StreamDisconnected {
        provider: String,
        reason: Option<String>,
    },

    // ========== Message Events ==========
    /// Text content delta from LLM stream
    TextDelta(String),

    /// Complete text message received
    TextComplete(String),

    /// Tool call started
    ToolCallStart {
        id: String,
        name: String,
    },

    /// Tool call progress update
    ToolCallProgress {
        id: String,
        message: String,
    },

    /// Tool call completed
    ToolCallComplete {
        id: String,
        success: bool,
        result: Option<String>,
        error: Option<String>,
    },

    // ========== Agent Events ==========
    /// Agent started processing
    AgentStarted {
        agent_id: String,
        task: String,
    },

    /// Agent state changed
    AgentStateChanged {
        agent_id: String,
        from: String,
        to: String,
    },

    /// Agent iteration started (for reactive agents)
    AgentIterationStart {
        agent_id: String,
        iteration: u32,
    },

    /// Agent completed
    AgentCompleted {
        agent_id: String,
        success: bool,
    },

    // ========== Session Events ==========
    /// New session created
    SessionCreated {
        session_id: String,
    },

    /// Session ended
    SessionEnded {
        session_id: String,
        reason: String,
    },

    // ========== Error Events ==========
    /// Error occurred
    Error {
        source: String,
        message: String,
        recoverable: bool,
    },

    /// Warning
    Warning {
        source: String,
        message: String,
    },

    // ========== System Events ==========
    /// System is shutting down
    Shutdown,

    /// Heartbeat for keep-alive
    Heartbeat,

    /// Custom event for extensibility
    Custom {
        name: String,
        data: serde_json::Value,
    },
}

impl Event {
    /// Create a text delta event
    pub fn text_delta(content: impl Into<String>) -> Self {
        Self::TextDelta(content.into())
    }

    /// Create a tool call start event
    pub fn tool_start(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::ToolCallStart {
            id: id.into(),
            name: name.into(),
        }
    }

    /// Create a tool call complete event
    pub fn tool_complete(id: impl Into<String>, success: bool, result: Option<String>) -> Self {
        Self::ToolCallComplete {
            id: id.into(),
            success,
            result,
            error: None,
        }
    }

    /// Create a tool call error event
    pub fn tool_error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ToolCallComplete {
            id: id.into(),
            success: false,
            result: None,
            error: Some(error.into()),
        }
    }

    /// Create an error event
    pub fn error(source: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self::Error {
            source: source.into(),
            message: message.into(),
            recoverable,
        }
    }

    /// Create a custom event
    pub fn custom(name: impl Into<String>, data: serde_json::Value) -> Self {
        Self::Custom {
            name: name.into(),
            data,
        }
    }

    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::StreamConnected { .. } => "stream_connected",
            Self::StreamDisconnected { .. } => "stream_disconnected",
            Self::TextDelta(_) => "text_delta",
            Self::TextComplete(_) => "text_complete",
            Self::ToolCallStart { .. } => "tool_call_start",
            Self::ToolCallProgress { .. } => "tool_call_progress",
            Self::ToolCallComplete { .. } => "tool_call_complete",
            Self::AgentStarted { .. } => "agent_started",
            Self::AgentStateChanged { .. } => "agent_state_changed",
            Self::AgentIterationStart { .. } => "agent_iteration_start",
            Self::AgentCompleted { .. } => "agent_completed",
            Self::SessionCreated { .. } => "session_created",
            Self::SessionEnded { .. } => "session_ended",
            Self::Error { .. } => "error",
            Self::Warning { .. } => "warning",
            Self::Shutdown => "shutdown",
            Self::Heartbeat => "heartbeat",
            Self::Custom { .. } => "custom",
        }
    }
}

/// Event bus for system-wide event distribution
///
/// The EventBus uses a broadcast channel to distribute events to multiple subscribers.
/// Each subscriber receives a copy of every published event.
///
/// # Example
///
/// ```rust
/// use sage_core::events::{EventBus, Event};
///
/// #[tokio::main]
/// async fn main() {
///     let bus = EventBus::new(100);
///
///     // Subscribe to events
///     let mut subscriber = bus.subscribe();
///
///     // Publish an event
///     bus.publish(Event::TextDelta("Hello".into()));
///
///     // Receive the event
///     let event = subscriber.recv().await.unwrap();
///     println!("Received: {:?}", event);
/// }
/// ```
#[derive(Debug)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    capacity: usize,
}

impl EventBus {
    /// Create a new event bus with the specified capacity
    ///
    /// The capacity determines how many events can be buffered before
    /// slow subscribers start losing events.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender, capacity }
    }

    /// Publish an event to all subscribers
    ///
    /// Returns the number of active receivers that will receive this event.
    /// Returns 0 if there are no active subscribers.
    pub fn publish(&self, event: Event) -> usize {
        match self.sender.send(event) {
            Ok(n) => n,
            Err(_) => 0, // No active receivers
        }
    }

    /// Subscribe to events
    ///
    /// Returns a receiver that will receive all future events.
    /// Events published before subscribing are not received.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get the channel capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Publish multiple events
    pub fn publish_all(&self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            self.publish(event);
        }
    }
}

impl Default for EventBus {
    /// Create a default event bus with capacity of 256 events
    fn default() -> Self {
        Self::new(256)
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            capacity: self.capacity,
        }
    }
}

/// Thread-safe wrapper around EventBus
pub type SharedEventBus = Arc<EventBus>;

/// Create a new shared event bus
pub fn shared_event_bus(capacity: usize) -> SharedEventBus {
    Arc::new(EventBus::new(capacity))
}

/// Event filter for selective subscription
pub struct EventFilter {
    types: Option<Vec<&'static str>>,
}

impl EventFilter {
    /// Create a new event filter
    pub fn new() -> Self {
        Self { types: None }
    }

    /// Filter by event types
    pub fn only_types(mut self, types: Vec<&'static str>) -> Self {
        self.types = Some(types);
        self
    }

    /// Check if an event matches the filter
    pub fn matches(&self, event: &Event) -> bool {
        match &self.types {
            Some(types) => types.contains(&event.event_type()),
            None => true,
        }
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Filtered event subscriber
pub struct FilteredSubscriber {
    receiver: broadcast::Receiver<Event>,
    filter: EventFilter,
}

impl FilteredSubscriber {
    /// Create a new filtered subscriber
    pub fn new(bus: &EventBus, filter: EventFilter) -> Self {
        Self {
            receiver: bus.subscribe(),
            filter,
        }
    }

    /// Receive the next matching event
    pub async fn recv(&mut self) -> Result<Event, broadcast::error::RecvError> {
        loop {
            let event = self.receiver.recv().await?;
            if self.filter.matches(&event) {
                return Ok(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_basic() {
        let bus = EventBus::new(100);
        let mut subscriber = bus.subscribe();

        // Publish an event
        let sent = bus.publish(Event::TextDelta("Hello".into()));
        assert_eq!(sent, 1);

        // Receive the event
        let event = subscriber.recv().await.unwrap();
        match event {
            Event::TextDelta(text) => assert_eq!(text, "Hello"),
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new(100);
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        bus.publish(Event::text_delta("Test"));

        // Both subscribers should receive the event
        let event1 = sub1.recv().await.unwrap();
        let event2 = sub2.recv().await.unwrap();

        assert!(matches!(event1, Event::TextDelta(_)));
        assert!(matches!(event2, Event::TextDelta(_)));
    }

    #[tokio::test]
    async fn test_no_subscribers() {
        let bus = EventBus::new(100);

        // Publishing with no subscribers should return 0
        let sent = bus.publish(Event::Heartbeat);
        assert_eq!(sent, 0);
    }

    #[tokio::test]
    async fn test_subscriber_count() {
        let bus = EventBus::new(100);
        assert_eq!(bus.subscriber_count(), 0);

        let _sub1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _sub2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[tokio::test]
    async fn test_event_types() {
        let events = vec![
            Event::StreamConnected {
                provider: "openai".into(),
                model: "gpt-4".into(),
            },
            Event::TextDelta("Hello".into()),
            Event::tool_start("1", "read_file"),
            Event::tool_complete("1", true, Some("content".into())),
            Event::error("test", "test error", true),
            Event::Shutdown,
        ];

        let types: Vec<_> = events.iter().map(|e| e.event_type()).collect();
        assert_eq!(
            types,
            vec![
                "stream_connected",
                "text_delta",
                "tool_call_start",
                "tool_call_complete",
                "error",
                "shutdown"
            ]
        );
    }

    #[tokio::test]
    async fn test_event_filter() {
        let filter = EventFilter::new().only_types(vec!["text_delta", "error"]);

        assert!(filter.matches(&Event::TextDelta("test".into())));
        assert!(filter.matches(&Event::error("src", "msg", true)));
        assert!(!filter.matches(&Event::Heartbeat));
    }

    #[tokio::test]
    async fn test_filtered_subscriber() {
        let bus = EventBus::new(100);
        let filter = EventFilter::new().only_types(vec!["text_delta"]);
        let mut subscriber = FilteredSubscriber::new(&bus, filter);

        // Publish multiple events
        bus.publish(Event::Heartbeat);
        bus.publish(Event::TextDelta("Hello".into()));
        bus.publish(Event::Heartbeat);

        // Should only receive the text delta
        let event = subscriber.recv().await.unwrap();
        assert!(matches!(event, Event::TextDelta(_)));
    }

    #[tokio::test]
    async fn test_clone_bus() {
        let bus1 = EventBus::new(100);
        let bus2 = bus1.clone();

        let mut sub = bus1.subscribe();

        // Publishing on cloned bus should reach original subscribers
        bus2.publish(Event::Heartbeat);

        let event = sub.recv().await.unwrap();
        assert!(matches!(event, Event::Heartbeat));
    }

    #[tokio::test]
    async fn test_shared_event_bus() {
        let bus = shared_event_bus(100);
        let bus_clone = bus.clone();

        let mut sub = bus.subscribe();
        bus_clone.publish(Event::Heartbeat);

        let event = sub.recv().await.unwrap();
        assert!(matches!(event, Event::Heartbeat));
    }
}
