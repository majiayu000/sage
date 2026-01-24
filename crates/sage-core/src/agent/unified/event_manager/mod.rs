//! Event management for unified executor
//!
//! Provides a centralized event dispatch system that coordinates UI updates
//! through the UI abstraction layer (UiContext).

#[cfg(test)]
mod tests;

use crate::ui::bridge::AgentEvent;
use crate::ui::traits::UiContext;

/// Execution events that can be observed throughout the agent lifecycle
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// A new execution step has started
    StepStarted { step_number: u32 },
    /// A tool execution has started
    ToolExecutionStarted { tool_name: String, tool_id: String },
    /// A tool execution has completed
    ToolExecutionCompleted {
        tool_name: String,
        tool_id: String,
        success: bool,
        duration_ms: u64,
    },
    /// A message was received from the LLM
    MessageReceived {
        content_length: usize,
        has_tool_calls: bool,
    },
    /// A session has started
    SessionStarted {
        session_id: String,
        model: String,
        provider: String,
    },
    /// A session has ended
    SessionEnded { session_id: String },
    /// The agent has started thinking
    ThinkingStarted { step_number: u32 },
    /// The agent has stopped thinking
    ThinkingStopped,
    /// An error occurred during execution
    ErrorOccurred { error_type: String, message: String },
}

/// Event manager that dispatches events to the UI
///
/// Uses `UiContext` for dependency injection instead of global state.
pub struct EventManager {
    pub(crate) current_step: u32,
    debug_events: bool,
    pub(crate) is_animating: bool,
    /// UI context for emitting events (replaces global emit_event)
    ui_context: UiContext,
}

impl EventManager {
    /// Create a new event manager with default (no-op) UI context
    pub fn new() -> Self {
        Self {
            current_step: 0,
            debug_events: false,
            is_animating: false,
            ui_context: UiContext::noop(),
        }
    }

    /// Create a new event manager with a custom UI context
    pub fn with_ui_context(ui_context: UiContext) -> Self {
        Self {
            current_step: 0,
            debug_events: false,
            is_animating: false,
            ui_context,
        }
    }

    /// Create event manager with debug logging enabled
    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.debug_events = enabled;
        self
    }

    /// Set the UI context
    pub fn set_ui_context(&mut self, ui_context: UiContext) {
        self.ui_context = ui_context;
    }

    /// Get a reference to the UI context
    pub fn ui_context(&self) -> &UiContext {
        &self.ui_context
    }

    /// Emit an event to the UI
    fn emit_ui_event(&self, event: AgentEvent) {
        self.ui_context.emit(event);
    }

    /// Emit an execution event
    pub async fn emit(&mut self, event: ExecutionEvent) {
        if self.debug_events {
            tracing::debug!("Event: {:?}", event);
        }

        match event {
            ExecutionEvent::StepStarted { step_number } => {
                self.current_step = step_number;
            }
            ExecutionEvent::ThinkingStarted { step_number } => {
                self.current_step = step_number;
                self.is_animating = true;
                self.emit_ui_event(AgentEvent::ThinkingStarted);
            }
            ExecutionEvent::ThinkingStopped => {
                self.is_animating = false;
                self.emit_ui_event(AgentEvent::ThinkingStopped);
            }
            ExecutionEvent::ToolExecutionStarted {
                ref tool_name,
                ref tool_id,
            } => {
                self.is_animating = true;
                self.emit_ui_event(AgentEvent::ToolExecutionStarted {
                    tool_name: tool_name.clone(),
                    tool_id: tool_id.clone(),
                    description: tool_name.clone(),
                });
            }
            ExecutionEvent::ToolExecutionCompleted {
                ref tool_name,
                ref tool_id,
                success,
                duration_ms,
            } => {
                self.is_animating = false;
                self.emit_ui_event(AgentEvent::ToolExecutionCompleted {
                    tool_name: tool_name.clone(),
                    tool_id: tool_id.clone(),
                    success,
                    duration_ms,
                    result_preview: None,
                });
            }
            ExecutionEvent::SessionStarted {
                ref session_id,
                ref model,
                ref provider,
            } => {
                tracing::info!("Session started: {} ({}/{})", session_id, provider, model);
                self.emit_ui_event(AgentEvent::SessionStarted {
                    session_id: session_id.clone(),
                    model: model.clone(),
                    provider: provider.clone(),
                });
            }
            ExecutionEvent::SessionEnded { ref session_id } => {
                tracing::info!("Session ended: {}", session_id);
                self.is_animating = false;
                self.emit_ui_event(AgentEvent::SessionEnded {
                    session_id: session_id.clone(),
                });
            }
            ExecutionEvent::ErrorOccurred {
                ref error_type,
                ref message,
            } => {
                tracing::error!("Execution error [{}]: {}", error_type, message);
                self.is_animating = false;
                self.emit_ui_event(AgentEvent::ErrorOccurred {
                    error_type: error_type.clone(),
                    message: message.clone(),
                });
            }
            ExecutionEvent::MessageReceived { .. } => {
                // Message received events are for logging/debugging only
            }
        }
    }

    /// Emit an execution event with custom detail
    pub async fn emit_with_detail(&mut self, event: ExecutionEvent, detail: String) {
        if self.debug_events {
            tracing::debug!("Event: {:?}, detail: {}", event, detail);
        }

        match event {
            ExecutionEvent::ToolExecutionStarted {
                ref tool_name,
                ref tool_id,
            } => {
                self.is_animating = true;
                self.emit_ui_event(AgentEvent::ToolExecutionStarted {
                    tool_name: tool_name.clone(),
                    tool_id: tool_id.clone(),
                    description: detail,
                });
            }
            _ => {
                self.emit(event).await;
            }
        }
    }

    /// Stop any running animation
    pub async fn stop_animation(&self) {
        self.emit_ui_event(AgentEvent::ThinkingStopped);
    }

    /// Set the current step number
    pub fn set_step(&mut self, step: u32) {
        self.current_step = step;
    }

    /// Set max steps for progress display
    pub fn set_max_steps(&self, _max: Option<u32>) {
        // No-op for now, the new UI handles this through state
    }

    /// Check if animation is currently running
    pub fn is_running(&self) -> bool {
        self.is_animating
    }

    /// Start thinking animation
    pub async fn start_animation_thinking(&mut self, step: u32) {
        self.current_step = step;
        self.is_animating = true;
        self.emit_ui_event(AgentEvent::ThinkingStarted);
    }

    /// Start tool execution animation
    pub async fn start_animation_tool(&mut self, tool_name: &str) {
        self.is_animating = true;
        self.emit_ui_event(AgentEvent::ToolExecutionStarted {
            tool_name: tool_name.to_string(),
            tool_id: String::new(),
            description: tool_name.to_string(),
        });
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}
