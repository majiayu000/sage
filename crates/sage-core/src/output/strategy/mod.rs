//! Output strategy pattern for flexible display modes
//!
//! This module implements the Strategy Pattern to decouple output display logic,
//! allowing runtime switching between different output modes.

mod batch;
mod json;
mod rnk;
mod silent;
mod streaming;

pub use batch::BatchOutput;
pub use json::JsonOutput;
pub use rnk::RnkOutput;
pub use silent::SilentOutput;
pub use streaming::StreamingOutput;

/// Output strategy trait for different display modes
///
/// Implementations of this trait control how LLM responses and tool executions
/// are displayed to the user.
pub trait OutputStrategy: Send + Sync {
    /// Called when content streaming starts (first chunk arrives)
    fn on_content_start(&self);

    /// Called for each content chunk during streaming
    fn on_content_chunk(&self, chunk: &str);

    /// Called when content streaming ends
    fn on_content_end(&self);

    /// Called when a tool execution starts
    fn on_tool_start(&self, name: &str, params: &str);

    /// Called when a tool execution completes
    fn on_tool_result(&self, success: bool, output: Option<&str>, error: Option<&str>);

    /// Called for thinking/status updates
    fn on_thinking(&self, message: &str);

    /// Called to stop thinking indicator
    fn on_thinking_stop(&self);

    /// Get the final collected content (for batch mode)
    fn get_collected_content(&self) -> Option<String> {
        None
    }
}

/// Output mode configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Real-time streaming output (default)
    #[default]
    Streaming,
    /// Batch output (collect then display)
    Batch,
    /// JSON output for SDK/API
    Json,
    /// Silent (no output)
    Silent,
    /// Rnk declarative UI output
    Rnk,
}

impl OutputMode {
    /// Create the appropriate output strategy for this mode
    pub fn create_strategy(&self) -> Box<dyn OutputStrategy> {
        match self {
            OutputMode::Streaming => Box::new(StreamingOutput::new()),
            OutputMode::Batch => Box::new(BatchOutput::new()),
            OutputMode::Json => Box::new(JsonOutput::new()),
            OutputMode::Silent => Box::new(SilentOutput::new()),
            OutputMode::Rnk => Box::new(RnkOutput::new()),
        }
    }
}

impl std::str::FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "streaming" | "stream" => Ok(OutputMode::Streaming),
            "batch" => Ok(OutputMode::Batch),
            "json" => Ok(OutputMode::Json),
            "silent" | "quiet" => Ok(OutputMode::Silent),
            "rnk" | "ui" => Ok(OutputMode::Rnk),
            _ => Err(format!("Unknown output mode: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_output() {
        let output = StreamingOutput::new();
        output.on_content_start();
        output.on_content_chunk("Hello ");
        output.on_content_chunk("World!");
        output.on_content_end();
    }

    #[test]
    fn test_batch_output() {
        let output = BatchOutput::new();
        output.on_content_start();
        output.on_content_chunk("Hello ");
        output.on_content_chunk("World!");
        output.on_content_end();
    }

    #[test]
    fn test_json_output() {
        let output = JsonOutput::new();
        output.on_content_start();
        output.on_content_chunk("Hello ");
        output.on_content_chunk("World!");
        output.on_content_end();
    }

    #[test]
    fn test_silent_output() {
        let output = SilentOutput::new();
        output.on_content_start();
        output.on_content_chunk("Hello ");
        output.on_content_chunk("World!");
        output.on_content_end();
    }
}
