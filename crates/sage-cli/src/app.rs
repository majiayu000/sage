//! Sage CLI Main Application (rnk-based)
//!
//! This is the new declarative UI application using rnk framework.

use rnk::core::Dimension;
use rnk::prelude::*;
use sage_core::ui::{
    bridge::{AgentEvent, AppState, EventAdapter, ExecutionPhase},
    components::{InputBox, MessageList, StatusBar, ThinkingIndicator, ToolExecutionView},
    theme::Icons,
};
use std::sync::{Arc, RwLock};

/// Sage CLI main application component
pub fn sage_app(state: Arc<RwLock<AppState>>) -> Element {
    // Get current state snapshot
    let current_state = state.read().unwrap().clone();

    // Build the UI
    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(Dimension::Percent(100.0))
        // Status bar at top
        .child(
            StatusBar::new(current_state.session.clone(), current_state.phase.clone()).render(),
        )
        // Message area (scrollable)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .overflow_y(Overflow::Scroll)
                // Message list
                .child(MessageList(current_state.display_messages()))
                // Thinking indicator
                .child(if let Some(thinking) = current_state.thinking.clone() {
                    ThinkingIndicator::new(thinking).render()
                } else {
                    Box::new().into_element()
                })
                // Tool execution
                .child(if let Some(tool) = current_state.tool_execution.clone() {
                    ToolExecutionView::new(tool).render()
                } else {
                    Box::new().into_element()
                })
                .into_element(),
        )
        // Input box at bottom
        .child(InputBox::new(current_state.input.clone()).render())
        .into_element()
}

/// Application state holder for the rnk app
pub struct SageAppState {
    pub adapter: EventAdapter,
    pub state: Arc<RwLock<AppState>>,
}

impl SageAppState {
    /// Create a new app state
    pub fn new() -> Self {
        let adapter = EventAdapter::with_default_state();
        let state = adapter.state_handle();
        Self { adapter, state }
    }

    /// Initialize with session info
    pub fn with_session(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        self.adapter.handle_event(AgentEvent::session_started(
            uuid::Uuid::new_v4().to_string(),
            model,
            provider,
        ));
        self
    }

    /// Get the event adapter for Agent to use
    pub fn event_adapter(&self) -> EventAdapter {
        self.adapter.clone()
    }

    /// Get the state for rendering
    pub fn state(&self) -> Arc<RwLock<AppState>> {
        Arc::clone(&self.state)
    }
}

impl Default for SageAppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the Sage CLI application with rnk
pub fn run_app() -> std::io::Result<()> {
    // Initialize icons from environment
    Icons::init_from_env();

    // Create app state
    let app_state = SageAppState::new().with_session("claude-sonnet-4-20250514", "anthropic");

    let state = app_state.state();

    // Run the rnk app
    render(move || sage_app(Arc::clone(&state)))
}

/// Simple demo to test the UI components
pub fn run_demo() -> std::io::Result<()> {
    use sage_core::ui::bridge::{Message, MessageContent, Role, SessionState};
    use std::time::Instant;

    Icons::init_from_env();

    let app_state = SageAppState::new();

    // Add some demo messages
    {
        let mut state = app_state.state.write().unwrap();
        state.session = SessionState {
            session_id: Some("demo-session".to_string()),
            model: "claude-sonnet-4-20250514".to_string(),
            provider: "anthropic".to_string(),
            working_dir: "/Users/demo/project".to_string(),
            git_branch: Some("main".to_string()),
            step: 1,
            max_steps: Some(10),
        };

        state.messages.push(Message {
            role: Role::User,
            content: MessageContent::Text("Help me refactor the UI code".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        });

        state.messages.push(Message {
            role: Role::Assistant,
            content: MessageContent::Text(
                "I'll help you refactor the UI code. Let me first analyze the current structure."
                    .to_string(),
            ),
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        });

        state.phase = ExecutionPhase::Idle;
    }

    let state = app_state.state();
    render(move || sage_app(Arc::clone(&state)))
}
