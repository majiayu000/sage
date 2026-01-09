//! Event management for unified executor
//!
//! Provides a centralized event dispatch system that coordinates:
//! - UI animations through AnimationManager
//! - Event logging and debugging
//!
//! This module unifies event handling that was previously scattered across
//! multiple locations in the codebase.

#[cfg(test)]
mod tests;

use crate::ui::animation::{AnimationContext, AnimationManager, AnimationState};

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

/// Event manager that dispatches events to UI and optional listeners
///
/// Wraps `AnimationManager` and provides a unified interface for event handling.
/// This centralizes event dispatch logic that was previously scattered across
/// the executor implementation.
pub struct EventManager {
    animation_manager: AnimationManager,
    current_step: u32,
    debug_events: bool,
}

impl EventManager {
    /// Create a new event manager
    pub fn new() -> Self {
        Self {
            animation_manager: AnimationManager::new(),
            current_step: 0,
            debug_events: false,
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
                self.animation_manager.set_step(step_number);
            }
            ExecutionEvent::ThinkingStarted { step_number } => {
                self.current_step = step_number;
                let context = AnimationContext::new().with_step(step_number);
                self.animation_manager
                    .start_with_context(AnimationState::Thinking, "Thinking", "blue", context)
                    .await;
            }
            ExecutionEvent::ThinkingStopped => {
                self.animation_manager.stop_animation().await;
            }
            ExecutionEvent::ToolExecutionStarted { tool_name, .. } => {
                let context = AnimationContext::new()
                    .with_step(self.current_step)
                    .with_detail(&tool_name);
                self.animation_manager
                    .start_with_context(
                        AnimationState::ExecutingTools,
                        &format!("Running {}", tool_name),
                        "green",
                        context,
                    )
                    .await;
            }
            ExecutionEvent::ToolExecutionCompleted { .. } => {
                self.animation_manager.stop_animation().await;
            }
            ExecutionEvent::SessionStarted { ref session_id } => {
                tracing::info!("Session started: {}", session_id);
            }
            ExecutionEvent::SessionEnded { ref session_id } => {
                tracing::info!("Session ended: {}", session_id);
                self.animation_manager.stop_animation().await;
            }
            ExecutionEvent::ErrorOccurred {
                ref error_type,
                ref message,
            } => {
                tracing::error!("Execution error [{}]: {}", error_type, message);
                self.animation_manager.stop_animation().await;
            }
            ExecutionEvent::MessageReceived { .. } => {
                // Message received events are for logging/debugging only
            }
        }
    }

    /// Stop any running animation
    pub async fn stop_animation(&self) {
        self.animation_manager.stop_animation().await;
    }

    /// Set the current step number
    pub fn set_step(&mut self, step: u32) {
        self.current_step = step;
        self.animation_manager.set_step(step);
    }

    /// Set max steps for progress display
    pub fn set_max_steps(&self, max: Option<u32>) {
        self.animation_manager.set_max_steps(max);
    }

    /// Check if animation is currently running
    pub fn is_animating(&self) -> bool {
        self.animation_manager.is_running()
    }

    /// Get the current animation state
    pub async fn animation_state(&self) -> AnimationState {
        self.animation_manager.current_state().await
    }

    /// Direct access to animation manager for advanced use cases
    pub fn animation_manager(&self) -> &AnimationManager {
        &self.animation_manager
    }

    /// Start animation directly (for backward compatibility)
    pub async fn start_animation(&self, state: AnimationState, message: &str, color: &str) {
        self.animation_manager
            .start_animation(state, message, color)
            .await;
    }

    /// Start animation with context (for backward compatibility)
    pub async fn start_with_context(
        &self,
        state: AnimationState,
        message: &str,
        color: &str,
        context: AnimationContext,
    ) {
        self.animation_manager
            .start_with_context(state, message, color, context)
            .await;
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}
