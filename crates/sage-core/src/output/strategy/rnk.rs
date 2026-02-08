//! Rnk UI output strategy.

use super::OutputStrategy;
use std::sync::Mutex;

/// Rnk UI output - sends events to the new declarative UI
///
/// Uses the event bridge to update AppState, which rnk then renders.
/// This replaces terminal print! calls with state updates.
///
/// **Note**: This implementation uses the deprecated `emit_event` function.
/// In a future version, this should be refactored to use `UiContext` instead.
#[derive(Debug, Default)]
pub struct RnkOutput {
    content_buffer: Mutex<String>,
}

impl RnkOutput {
    pub fn new() -> Self {
        Self {
            content_buffer: Mutex::new(String::new()),
        }
    }
}

#[allow(deprecated)]
impl OutputStrategy for RnkOutput {
    fn on_content_start(&self) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.clear();
        }
        crate::ui::bridge::emit_event(crate::ui::bridge::AgentEvent::ContentStreamStarted);
    }

    fn on_content_chunk(&self, chunk: &str) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.push_str(chunk);
        }
        crate::ui::bridge::emit_event(crate::ui::bridge::AgentEvent::ContentChunk {
            chunk: chunk.to_string(),
        });
    }

    fn on_content_end(&self) {
        crate::ui::bridge::emit_event(crate::ui::bridge::AgentEvent::ContentStreamEnded);
    }

    fn on_tool_start(&self, _name: &str, _params: &str) {
        // No-op: tool events are emitted by tool_display::display_tool_start via EventManager
        // to avoid duplicate ToolExecutionStarted events
    }

    fn on_tool_result(&self, _success: bool, _output: Option<&str>, _error: Option<&str>) {
        // No-op: tool events are emitted by tool_display::display_tool_result via EventManager
        // to avoid duplicate ToolExecutionCompleted events
    }

    fn on_thinking(&self, _message: &str) {
        crate::ui::bridge::emit_event(crate::ui::bridge::AgentEvent::ThinkingStarted);
    }

    fn on_thinking_stop(&self) {
        crate::ui::bridge::emit_event(crate::ui::bridge::AgentEvent::ThinkingStopped);
    }

    fn get_collected_content(&self) -> Option<String> {
        self.content_buffer.lock().ok().map(|b| b.clone())
    }
}
