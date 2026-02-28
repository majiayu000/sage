//! UI event output strategy.
//!
//! Emits streaming/thinking updates through `UiContext` instead of using
//! deprecated global event bridges.

use super::OutputStrategy;
use crate::ui::bridge::AgentEvent;
use crate::ui::traits::UiContext;
use std::sync::Mutex;

/// UI event output strategy backed by an injected `UiContext`.
#[derive(Clone, Debug)]
pub struct UiEventOutput {
    ui_context: UiContext,
    content_buffer: std::sync::Arc<Mutex<String>>,
}

impl UiEventOutput {
    /// Create a new UI event output strategy.
    pub fn new(ui_context: UiContext) -> Self {
        Self {
            ui_context,
            content_buffer: std::sync::Arc::new(Mutex::new(String::new())),
        }
    }
}

impl OutputStrategy for UiEventOutput {
    fn on_content_start(&self) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.clear();
        }
        self.ui_context.emit(AgentEvent::ContentStreamStarted);
    }

    fn on_content_chunk(&self, chunk: &str) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.push_str(chunk);
        }
        self.ui_context.emit(AgentEvent::ContentChunk {
            chunk: chunk.to_string(),
        });
    }

    fn on_content_end(&self) {
        self.ui_context.emit(AgentEvent::ContentStreamEnded);
    }

    fn on_tool_start(&self, _name: &str, _params: &str) {
        // No-op: tool events are emitted by EventManager to avoid duplicates.
    }

    fn on_tool_result(&self, _success: bool, _output: Option<&str>, _error: Option<&str>) {
        // No-op: tool events are emitted by EventManager to avoid duplicates.
    }

    fn on_thinking(&self, _message: &str) {
        self.ui_context.emit(AgentEvent::ThinkingStarted);
    }

    fn on_thinking_stop(&self) {
        self.ui_context.emit(AgentEvent::ThinkingStopped);
    }

    fn get_collected_content(&self) -> Option<String> {
        self.content_buffer.lock().ok().map(|b| b.clone())
    }
}
