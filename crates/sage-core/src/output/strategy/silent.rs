//! Silent output strategy.

use super::OutputStrategy;

/// Silent output - produces no output
///
/// Useful for:
/// - Testing
/// - Background tasks
/// - When output is handled elsewhere
#[derive(Debug, Default, Clone, Copy)]
pub struct SilentOutput;

impl SilentOutput {
    pub fn new() -> Self {
        Self
    }
}

impl OutputStrategy for SilentOutput {
    fn on_content_start(&self) {}
    fn on_content_chunk(&self, _chunk: &str) {}
    fn on_content_end(&self) {}
    fn on_tool_start(&self, _name: &str, _params: &str) {}
    fn on_tool_result(&self, _success: bool, _output: Option<&str>, _error: Option<&str>) {}
    fn on_thinking(&self, _message: &str) {}
    fn on_thinking_stop(&self) {}
}
