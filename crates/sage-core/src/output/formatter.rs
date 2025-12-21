//! Output formatters
//!
//! This module provides formatters for different output modes.

use std::io::Write;

use super::types::{JsonOutput, OutputEvent, OutputFormat};

/// Output formatter trait
pub trait OutputFormatter: Send + Sync {
    /// Format an output event
    fn format_event(&self, event: &OutputEvent) -> String;

    /// Format the final output
    fn format_final(&self, output: &JsonOutput) -> String;

    /// Get the output format
    fn format(&self) -> OutputFormat;
}

/// Text formatter for human-readable output
pub struct TextFormatter {
    /// Whether to show timestamps
    show_timestamps: bool,
    /// Whether to use colors
    use_colors: bool,
}

impl TextFormatter {
    /// Create a new text formatter
    pub fn new() -> Self {
        Self {
            show_timestamps: false,
            use_colors: true,
        }
    }

    /// Show timestamps
    pub fn with_timestamps(mut self) -> Self {
        self.show_timestamps = true;
        self
    }

    /// Disable colors
    pub fn without_colors(mut self) -> Self {
        self.use_colors = false;
        self
    }

    /// Format with optional color
    fn colorize(&self, text: &str, color: &str) -> String {
        if self.use_colors {
            format!("\x1b[{}m{}\x1b[0m", color, text)
        } else {
            text.to_string()
        }
    }

    /// Format timestamp prefix
    fn timestamp_prefix(&self, event: &OutputEvent) -> String {
        if self.show_timestamps {
            format!("[{}] ", event.timestamp().format("%H:%M:%S"))
        } else {
            String::new()
        }
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for TextFormatter {
    fn format_event(&self, event: &OutputEvent) -> String {
        let prefix = self.timestamp_prefix(event);

        match event {
            OutputEvent::System(e) => {
                format!(
                    "{}{}",
                    prefix,
                    self.colorize(&format!("System: {}", e.message), "36")
                )
            }
            OutputEvent::Assistant(e) => {
                format!("{}{}", prefix, e.content)
            }
            OutputEvent::ToolCallStart(e) => {
                format!(
                    "{}{}",
                    prefix,
                    self.colorize(&format!("⚡ {} starting...", e.tool_name), "33")
                )
            }
            OutputEvent::ToolCallResult(e) => {
                let status = if e.success {
                    self.colorize("✓", "32")
                } else {
                    self.colorize("✗", "31")
                };
                let output = match (&e.output, &e.error) {
                    (Some(o), _) => format!(": {}", truncate(o, 100)),
                    (_, Some(err)) => format!(": {}", self.colorize(err, "31")),
                    _ => String::new(),
                };
                format!(
                    "{}{} {} ({}ms){}",
                    prefix, status, e.tool_name, e.duration_ms, output
                )
            }
            OutputEvent::UserPrompt(e) => {
                format!(
                    "{}{}",
                    prefix,
                    self.colorize(&format!("> {}", e.content), "34")
                )
            }
            OutputEvent::Error(e) => {
                format!(
                    "{}{}",
                    prefix,
                    self.colorize(&format!("Error: {}", e.message), "31")
                )
            }
            OutputEvent::Result(e) => {
                let mut output = format!("{}{}", prefix, e.content);
                if let Some(ref cost) = e.cost {
                    let mut token_info = format!(
                        "Tokens: {} in / {} out = {} total",
                        cost.input_tokens, cost.output_tokens, cost.total_tokens
                    );

                    // Add cache metrics if available
                    if let Some(cache_summary) = cost.cache_summary() {
                        token_info.push_str(&format!(" ({})", cache_summary));
                    }

                    output.push_str(&format!("\n{}", self.colorize(&token_info, "90")));
                }
                output
            }
        }
    }

    fn format_final(&self, output: &JsonOutput) -> String {
        let mut result = String::new();

        if output.success {
            result.push_str(&output.result);
        } else if let Some(ref error) = output.error {
            result.push_str(&self.colorize(&format!("Error: {}", error), "31"));
        }

        // Add summary
        if !output.tool_calls.is_empty() {
            result.push_str(&format!(
                "\n\n{}",
                self.colorize(
                    &format!("Made {} tool call(s)", output.tool_calls.len()),
                    "90"
                )
            ));
        }

        if let Some(ref cost) = output.cost {
            let mut token_info = format!(
                "Tokens: {} total ({}ms)",
                cost.total_tokens, output.duration_ms
            );

            // Add cache metrics if available
            if let Some(cache_summary) = cost.cache_summary() {
                token_info.push_str(&format!(" - {}", cache_summary));
            }

            result.push_str(&format!("\n{}", self.colorize(&token_info, "90")));
        }

        result
    }

    fn format(&self) -> OutputFormat {
        OutputFormat::Text
    }
}

/// JSON formatter for structured output
pub struct JsonFormatter {
    /// Pretty print JSON
    pretty: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new() -> Self {
        Self { pretty: false }
    }

    /// Enable pretty printing
    pub fn pretty(mut self) -> Self {
        self.pretty = true;
        self
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for JsonFormatter {
    fn format_event(&self, event: &OutputEvent) -> String {
        if self.pretty {
            serde_json::to_string_pretty(event).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string())
        }
    }

    fn format_final(&self, output: &JsonOutput) -> String {
        if self.pretty {
            serde_json::to_string_pretty(output).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string(output).unwrap_or_else(|_| "{}".to_string())
        }
    }

    fn format(&self) -> OutputFormat {
        OutputFormat::Json
    }
}

/// Stream JSON formatter (JSONL - one JSON per line)
pub struct StreamJsonFormatter;

impl StreamJsonFormatter {
    /// Create a new stream JSON formatter
    pub fn new() -> Self {
        Self
    }
}

impl Default for StreamJsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for StreamJsonFormatter {
    fn format_event(&self, event: &OutputEvent) -> String {
        // Each event is a single JSON line
        serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_final(&self, output: &JsonOutput) -> String {
        // Final output is also a single JSON line
        serde_json::to_string(output).unwrap_or_else(|_| "{}".to_string())
    }

    fn format(&self) -> OutputFormat {
        OutputFormat::StreamJson
    }
}

/// Create formatter for the specified format
pub fn create_formatter(format: OutputFormat) -> Box<dyn OutputFormatter> {
    match format {
        OutputFormat::Text => Box::new(TextFormatter::new()),
        OutputFormat::Json => Box::new(JsonFormatter::new().pretty()),
        OutputFormat::StreamJson => Box::new(StreamJsonFormatter::new()),
    }
}

/// Output writer that wraps a formatter and writer
pub struct OutputWriter<W: Write> {
    writer: W,
    formatter: Box<dyn OutputFormatter>,
}

impl<W: Write> OutputWriter<W> {
    /// Create a new output writer
    pub fn new(writer: W, format: OutputFormat) -> Self {
        Self {
            writer,
            formatter: create_formatter(format),
        }
    }

    /// Create with custom formatter
    pub fn with_formatter(writer: W, formatter: Box<dyn OutputFormatter>) -> Self {
        Self { writer, formatter }
    }

    /// Write an event
    pub fn write_event(&mut self, event: &OutputEvent) -> std::io::Result<()> {
        let formatted = self.formatter.format_event(event);
        writeln!(self.writer, "{}", formatted)?;
        self.writer.flush()
    }

    /// Write final output
    pub fn write_final(&mut self, output: &JsonOutput) -> std::io::Result<()> {
        let formatted = self.formatter.format_final(output);
        writeln!(self.writer, "{}", formatted)?;
        self.writer.flush()
    }

    /// Get the underlying formatter
    pub fn formatter(&self) -> &dyn OutputFormatter {
        self.formatter.as_ref()
    }

    /// Get the format
    pub fn format(&self) -> OutputFormat {
        self.formatter.format()
    }
}

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_formatter_system() {
        let formatter = TextFormatter::new().without_colors();
        let event = OutputEvent::system("Test message");
        let output = formatter.format_event(&event);
        assert!(output.contains("System: Test message"));
    }

    #[test]
    fn test_text_formatter_assistant() {
        let formatter = TextFormatter::new();
        let event = OutputEvent::assistant("Hello, world!");
        let output = formatter.format_event(&event);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_text_formatter_with_timestamps() {
        let formatter = TextFormatter::new().with_timestamps();
        let event = OutputEvent::system("Test");
        let output = formatter.format_event(&event);
        assert!(output.contains("["));
        assert!(output.contains("]"));
    }

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter::new();
        let event = OutputEvent::system("Test");
        let output = formatter.format_event(&event);
        assert!(output.contains("\"type\":\"system\""));
    }

    #[test]
    fn test_json_formatter_pretty() {
        let formatter = JsonFormatter::new().pretty();
        let event = OutputEvent::system("Test");
        let output = formatter.format_event(&event);
        assert!(output.contains('\n')); // Pretty print has newlines
    }

    #[test]
    fn test_stream_json_formatter() {
        let formatter = StreamJsonFormatter::new();
        let event = OutputEvent::system("Test");
        let output = formatter.format_event(&event);
        // Should be single line JSON
        assert!(!output.contains('\n'));
        assert!(output.contains("system"));
    }

    #[test]
    fn test_create_formatter() {
        let text = create_formatter(OutputFormat::Text);
        assert_eq!(text.format(), OutputFormat::Text);

        let json = create_formatter(OutputFormat::Json);
        assert_eq!(json.format(), OutputFormat::Json);

        let stream = create_formatter(OutputFormat::StreamJson);
        assert_eq!(stream.format(), OutputFormat::StreamJson);
    }

    #[test]
    fn test_output_writer() {
        let mut buffer = Vec::new();
        let mut writer = OutputWriter::new(&mut buffer, OutputFormat::StreamJson);

        writer.write_event(&OutputEvent::system("Test")).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("system"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long string", 10), "this is...");
    }

    #[test]
    fn test_text_formatter_tool_result_success() {
        let formatter = TextFormatter::new().without_colors();

        if let OutputEvent::ToolCallResult(mut e) = OutputEvent::tool_result("call_1", "Read", true)
        {
            e.duration_ms = 100;
            e.output = Some("File content".to_string());
            let event = OutputEvent::ToolCallResult(e);
            let output = formatter.format_event(&event);
            assert!(output.contains("Read"));
            assert!(output.contains("100ms"));
        }
    }

    #[test]
    fn test_text_formatter_error() {
        let formatter = TextFormatter::new().without_colors();
        let event = OutputEvent::error("Something failed");
        let output = formatter.format_event(&event);
        assert!(output.contains("Error: Something failed"));
    }

    #[test]
    fn test_json_output_format_final() {
        let formatter = JsonFormatter::new();
        let output = JsonOutput::success("Done").with_duration(1000);
        let json = formatter.format_final(&output);
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"duration_ms\":1000"));
    }

    #[test]
    fn test_text_format_final() {
        let formatter = TextFormatter::new().without_colors();
        let output = JsonOutput::success("All done!")
            .with_duration(500)
            .with_tool_call(super::super::types::ToolCallSummary::new(
                "call_1", "Read", true,
            ));
        let text = formatter.format_final(&output);
        assert!(text.contains("All done!"));
        assert!(text.contains("1 tool call"));
    }
}
