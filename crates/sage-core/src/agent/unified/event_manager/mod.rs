//! Event management for unified executor
//!
//! Provides a centralized event dispatch system that coordinates UI updates
//! through the rnk-based UI bridge.

#[cfg(test)]
mod tests;

use crate::ui::bridge::{emit_event, AgentEvent};

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
    SessionStarted { session_id: String },
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
pub struct EventManager {
    pub(crate) current_step: u32,
    debug_events: bool,
    pub(crate) is_animating: bool,
}

impl EventManager {
    /// Create a new event manager
    pub fn new() -> Self {
        Self {
            current_step: 0,
            debug_events: false,
            is_animating: false,
        }
    }

    /// Create event manager with debug logging enabled
    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.debug_events = enabled;
        self
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
                emit_event(AgentEvent::ThinkingStarted);
            }
            ExecutionEvent::ThinkingStopped => {
                self.is_animating = false;
                emit_event(AgentEvent::ThinkingStopped);
            }
            ExecutionEvent::ToolExecutionStarted {
                ref tool_name,
                ref tool_id,
            } => {
                self.is_animating = true;
                emit_event(AgentEvent::ToolExecutionStarted {
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
                emit_event(AgentEvent::ToolExecutionCompleted {
                    tool_name: tool_name.clone(),
                    tool_id: tool_id.clone(),
                    success,
                    duration_ms,
                    result_preview: None,
                });
            }
            ExecutionEvent::SessionStarted { ref session_id } => {
                tracing::info!("Session started: {}", session_id);
                emit_event(AgentEvent::SessionStarted {
                    session_id: session_id.clone(),
                    model: String::new(),
                    provider: String::new(),
                });
            }
            ExecutionEvent::SessionEnded { ref session_id } => {
                tracing::info!("Session ended: {}", session_id);
                self.is_animating = false;
                emit_event(AgentEvent::SessionEnded {
                    session_id: session_id.clone(),
                });
            }
            ExecutionEvent::ErrorOccurred {
                ref error_type,
                ref message,
            } => {
                tracing::error!("Execution error [{}]: {}", error_type, message);
                self.is_animating = false;
                emit_event(AgentEvent::ErrorOccurred {
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
                emit_event(AgentEvent::ToolExecutionStarted {
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
        emit_event(AgentEvent::ThinkingStopped);
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
        emit_event(AgentEvent::ThinkingStarted);
    }

    /// Start tool execution animation
    pub async fn start_animation_tool(&mut self, tool_name: &str) {
        self.is_animating = true;
        emit_event(AgentEvent::ToolExecutionStarted {
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
