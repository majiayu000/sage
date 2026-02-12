//! JSON output strategy.

use super::OutputStrategy;
use std::sync::Mutex;

/// JSON output - outputs structured JSON for SDK/API integration
///
/// Each event is output as a JSON object on a separate line (JSON Lines format).
/// Useful for:
/// - SDK/API consumers
/// - Piping to other tools
/// - Machine-readable output
#[derive(Debug, Default)]
pub struct JsonOutputStrategy {
    content_buffer: Mutex<String>,
}

impl JsonOutputStrategy {
    pub fn new() -> Self {
        Self {
            content_buffer: Mutex::new(String::new()),
        }
    }

    fn emit_event(&self, event_type: &str, data: serde_json::Value) {
        let event = serde_json::json!({
            "type": event_type,
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!("{}", serde_json::to_string(&event).unwrap_or_default());
    }
}

impl OutputStrategy for JsonOutputStrategy {
    fn on_content_start(&self) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.clear();
        }
        self.emit_event("content_start", serde_json::json!({}));
    }

    fn on_content_chunk(&self, chunk: &str) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.push_str(chunk);
        }
        self.emit_event("content_chunk", serde_json::json!({ "chunk": chunk }));
    }

    fn on_content_end(&self) {
        let content = self.content_buffer.lock().ok().map(|b| b.clone());
        self.emit_event("content_end", serde_json::json!({ "content": content }));
    }

    fn on_tool_start(&self, name: &str, params: &str) {
        self.emit_event(
            "tool_start",
            serde_json::json!({
                "name": name,
                "params": params
            }),
        );
    }

    fn on_tool_result(&self, success: bool, output: Option<&str>, error: Option<&str>) {
        self.emit_event(
            "tool_result",
            serde_json::json!({
                "success": success,
                "output": output,
                "error": error
            }),
        );
    }

    fn on_thinking(&self, message: &str) {
        self.emit_event("thinking", serde_json::json!({ "message": message }));
    }

    fn on_thinking_stop(&self) {
        self.emit_event("thinking_stop", serde_json::json!({}));
    }
}
