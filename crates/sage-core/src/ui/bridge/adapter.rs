//! Event Adapter - Converts Agent events to UI state updates
//!
//! This is the bridge between Agent execution and UI rendering.
//! All Agent events flow through here to update the AppState.

use super::events::AgentEvent;
use super::state::{AppState, ExecutionPhase};
use std::sync::{Arc, RwLock};
use tokio::sync::watch;

/// Adapter that converts Agent events to UI state updates
pub struct EventAdapter {
    state: Arc<RwLock<AppState>>,
    /// Channel sender for broadcasting state changes to subscribers
    state_tx: watch::Sender<AppState>,
}

impl EventAdapter {
    /// Create a new event adapter with the given state
    pub fn new(state: Arc<RwLock<AppState>>) -> Self {
        let initial_state = state.read().unwrap().clone();
        let (state_tx, _) = watch::channel(initial_state);
        Self { state, state_tx }
    }

    /// Create a new event adapter with default state
    pub fn with_default_state() -> Self {
        let state = AppState::default();
        let (state_tx, _) = watch::channel(state.clone());
        Self {
            state: Arc::new(RwLock::new(state)),
            state_tx,
        }
    }

    /// Subscribe to state changes
    ///
    /// Returns a receiver that will be notified whenever the state changes.
    /// Use `receiver.changed().await` to wait for updates.
    pub fn subscribe(&self) -> watch::Receiver<AppState> {
        self.state_tx.subscribe()
    }

    /// Get a clone of the state Arc for sharing
    pub fn state_handle(&self) -> Arc<RwLock<AppState>> {
        Arc::clone(&self.state)
    }

    /// Handle an agent event, updating the state accordingly
    pub fn handle_event(&self, event: AgentEvent) {
        let state_snapshot = {
            let mut state = self.state.write().unwrap();
            self.apply_event(&mut state, event);
            state.clone()
        };
        // Notify all subscribers of state change
        let _ = self.state_tx.send(state_snapshot);
    }

    /// Apply an event to the state
    fn apply_event(&self, state: &mut AppState, event: AgentEvent) {
        match event {
            AgentEvent::SessionStarted {
                session_id,
                model,
                provider,
            } => {
                state.session.session_id = Some(session_id);
                state.session.model = model;
                state.session.provider = provider;
            }

            AgentEvent::SessionEnded { .. } => {
                state.phase = ExecutionPhase::Idle;
            }

            AgentEvent::StepStarted { step_number } => {
                state.session.step = step_number;
            }

            AgentEvent::ThinkingStarted => {
                state.start_thinking();
            }

            AgentEvent::ThinkingStopped => {
                state.stop_thinking();
            }

            AgentEvent::ContentStreamStarted => {
                state.start_streaming();
            }

            AgentEvent::ContentChunk { chunk } => {
                state.append_streaming_chunk(&chunk);
            }

            AgentEvent::ContentStreamEnded => {
                state.finish_streaming();
            }

            AgentEvent::ToolExecutionStarted {
                tool_name,
                tool_id: _,
                description,
            } => {
                state.start_tool(tool_name, description);
            }

            AgentEvent::ToolExecutionCompleted {
                success,
                result_preview,
                ..
            } => {
                // Convert result_preview to the format expected by finish_tool
                let output = if success { result_preview.clone() } else { None };
                let error = if success { None } else { result_preview };
                state.finish_tool(success, output, error);
            }

            AgentEvent::UserInputReceived { input } => {
                state.add_user_message(input);
                state.input.text.clear();
                state.input.cursor_pos = 0;
            }

            AgentEvent::ErrorOccurred { message, .. } => {
                state.phase = ExecutionPhase::Error { message };
            }

            AgentEvent::UserInputRequested { prompt } => {
                state.phase = ExecutionPhase::WaitingConfirmation { prompt };
            }

            AgentEvent::GitBranchChanged { branch } => {
                state.session.git_branch = Some(branch);
            }

            AgentEvent::WorkingDirectoryChanged { path } => {
                state.session.working_dir = path;
            }
        }
    }

    /// Get a snapshot of the current state
    pub fn get_state(&self) -> AppState {
        self.state.read().unwrap().clone()
    }

    /// Update state with a closure and notify subscribers
    pub fn update_state<F>(&self, f: F)
    where
        F: FnOnce(&mut AppState),
    {
        let state_snapshot = {
            let mut state = self.state.write().unwrap();
            f(&mut state);
            state.clone()
        };
        let _ = self.state_tx.send(state_snapshot);
    }
}

impl Clone for EventAdapter {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            state_tx: self.state_tx.clone(),
        }
    }
}

/// Global event adapter instance for Agent to use
static GLOBAL_ADAPTER: std::sync::OnceLock<EventAdapter> = std::sync::OnceLock::new();

/// Set the global event adapter
pub fn set_global_adapter(adapter: EventAdapter) {
    let _ = GLOBAL_ADAPTER.set(adapter);
}

/// Get the global event adapter
pub fn global_adapter() -> Option<&'static EventAdapter> {
    GLOBAL_ADAPTER.get()
}

/// Emit an event to the global adapter
pub fn emit_event(event: AgentEvent) {
    if let Some(adapter) = global_adapter() {
        adapter.handle_event(event.clone());
        // Notify rnk to re-render after state update
        rnk::request_render();
    } else {
        tracing::warn!("emit_event called but no global adapter set: {:?}", event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_handles_thinking() {
        let adapter = EventAdapter::with_default_state();

        adapter.handle_event(AgentEvent::ThinkingStarted);
        let state = adapter.get_state();
        assert!(matches!(state.phase, ExecutionPhase::Thinking));

        adapter.handle_event(AgentEvent::ThinkingStopped);
        let state = adapter.get_state();
        assert!(matches!(state.phase, ExecutionPhase::Idle));
    }

    #[test]
    fn test_adapter_handles_streaming() {
        let adapter = EventAdapter::with_default_state();

        adapter.handle_event(AgentEvent::ContentStreamStarted);
        adapter.handle_event(AgentEvent::chunk("Hello "));
        adapter.handle_event(AgentEvent::chunk("World"));
        adapter.handle_event(AgentEvent::ContentStreamEnded);

        let state = adapter.get_state();
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_adapter_shared_state() {
        let adapter = EventAdapter::with_default_state();
        let state_handle = adapter.state_handle();

        adapter.handle_event(AgentEvent::ThinkingStarted);

        let state = state_handle.read().unwrap();
        assert!(matches!(state.phase, ExecutionPhase::Thinking));
    }

    #[test]
    fn test_error_event_sets_error_phase() {
        let adapter = EventAdapter::with_default_state();

        // Simulate the error flow from execution_loop.rs
        // 1. First ThinkingStopped is emitted (this sets phase to Idle)
        adapter.handle_event(AgentEvent::ThinkingStopped);
        let state = adapter.get_state();
        assert!(matches!(state.phase, ExecutionPhase::Idle), "After ThinkingStopped: {:?}", state.phase);

        // 2. Then error is emitted
        adapter.handle_event(AgentEvent::error("api_error", "Test error message"));

        // Verify state is Error
        let state = adapter.get_state();
        match &state.phase {
            ExecutionPhase::Error { message } => {
                assert_eq!(message, "Test error message");
            }
            other => {
                panic!("Expected Error phase, got {:?}", other);
            }
        }
    }

    #[test]
    fn test_error_not_overwritten_by_thinking_stopped() {
        let adapter = EventAdapter::with_default_state();

        // Set error state
        adapter.handle_event(AgentEvent::error("api_error", "Test error"));

        // Verify error is set
        let state = adapter.get_state();
        assert!(matches!(state.phase, ExecutionPhase::Error { .. }), "Error should be set");

        // ThinkingStopped should NOT overwrite error
        adapter.handle_event(AgentEvent::ThinkingStopped);

        let state = adapter.get_state();
        // This test will FAIL if ThinkingStopped overwrites Error
        assert!(matches!(state.phase, ExecutionPhase::Error { .. }),
            "Error should NOT be overwritten by ThinkingStopped, got {:?}", state.phase);
    }
}
