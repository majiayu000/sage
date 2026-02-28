//! Minimal example showing how an external GUI can consume sage-core events/state.

use sage_core::ui::{AgentEvent, AgentEventDto, AppStateDto, EventAdapter, EventSink, UiContext};
use std::sync::Arc;

struct GuiEventSink {
    adapter: Arc<EventAdapter>,
}

impl GuiEventSink {
    fn new(adapter: Arc<EventAdapter>) -> Self {
        Self { adapter }
    }
}

impl EventSink for GuiEventSink {
    fn handle_event(&self, event: AgentEvent) {
        let dto = AgentEventDto::from(&event);
        println!(
            "event: {}",
            serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string())
        );
        self.adapter.handle_event(event);
    }

    fn request_refresh(&self) {
        // In a real GUI, trigger redraw / push state over websocket here.
    }
}

fn main() {
    let adapter = Arc::new(EventAdapter::with_default_state());
    let sink = Arc::new(GuiEventSink::new(Arc::clone(&adapter)));
    let ui = UiContext::new(sink);

    ui.emit(AgentEvent::session_started(
        "demo-session",
        "demo-model",
        "demo-provider",
    ));
    ui.emit(AgentEvent::UserInputReceived {
        input: "Explain this repository".to_string(),
    });
    ui.emit(AgentEvent::ThinkingStarted);
    ui.emit(AgentEvent::ContentStreamStarted);
    ui.emit(AgentEvent::chunk("Hello from Sage backend"));
    ui.emit(AgentEvent::ContentStreamEnded);
    ui.emit(AgentEvent::ThinkingStopped);

    let snapshot = AppStateDto::from(adapter.get_state());
    println!(
        "state: {}",
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
    );
}
