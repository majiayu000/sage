//! Event Adapter - Converts Agent events to UI state updates
//!
//! This is the bridge between Agent execution and UI rendering.
//! All Agent events flow through here to update the AppState.

use super::events::AgentEvent;
use super::state::{AppState, ExecutionPhase};
use std::sync::{Arc, RwLock};

/// Adapter that converts Agent events to UI state updates
pub struct EventAdapter {
    state: Arc<RwLock<AppState>>,
}

impl EventAdapter {
    /// Create a new event adapter with the given state
    pub fn new(state: Arc<RwLock<AppState>>) -> Self {
        Self { state }
    }

    /// Create a new event adapter with default state
    pub fn with_default_state() -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
        }
    }

    /// Get a clone of the state Arc for sharing
    pub fn state_handle(&self) -> Arc<RwLock<AppState>> {
        Arc::clone(&self.state)
    }

    /// Handle an agent event, updating the state accordingly
    pub fn handle_event(&self, event: AgentEvent) {
        let mut state = self.state.write().unwrap();
        self.apply_event(&mut state, event);
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
                description,
            } => {
                state.start_tool(tool_name, description);
            }

            AgentEvent::ToolExecutionCompleted {
                success,
                output,
                error,
                ..
            } => {
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

    /// Update state with a closure
    pub fn update_state<F>(&self, f: F)
    where
        F: FnOnce(&mut AppState),
    {
        let mut state = self.state.write().unwrap();
        f(&mut state);
    }
}

impl Clone for EventAdapter {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
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
        adapter.handle_event(event);
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
}
