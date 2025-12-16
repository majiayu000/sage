//! Output type definitions
//!
//! This module defines types for structured output formatting,
//! supporting text, JSON, and streaming JSON modes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Output format mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable text (default)
    #[default]
    Text,
    /// Structured JSON output
    Json,
    /// JSONL streaming output (one JSON object per line)
    StreamJson,
}

impl OutputFormat {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "stream-json" | "streamjson" | "jsonl" => Some(Self::StreamJson),
            _ => None,
        }
    }

    /// Check if this format is JSON-based
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json | Self::StreamJson)
    }

    /// Check if this format is streaming
    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::StreamJson)
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
            Self::StreamJson => write!(f, "stream-json"),
        }
    }
}

/// Output event for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputEvent {
    /// System information
    System(SystemEvent),

    /// Assistant message/response
    Assistant(AssistantEvent),

    /// Tool call started
    ToolCallStart(ToolCallStartEvent),

    /// Tool call result
    ToolCallResult(ToolCallResultEvent),

    /// User prompt processed
    UserPrompt(UserPromptEvent),

    /// Error occurred
    Error(ErrorEvent),

    /// Result/completion
    Result(ResultEvent),
}

impl OutputEvent {
    /// Create a system event
    pub fn system(message: impl Into<String>) -> Self {
        Self::System(SystemEvent {
            message: message.into(),
            timestamp: Utc::now(),
        })
    }

    /// Create an assistant event
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant(AssistantEvent {
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// Create a tool call start event
    pub fn tool_start(call_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::ToolCallStart(ToolCallStartEvent {
            call_id: call_id.into(),
            tool_name: name.into(),
            arguments: serde_json::Value::Null,
            timestamp: Utc::now(),
        })
    }

    /// Create a tool call result event
    pub fn tool_result(call_id: impl Into<String>, name: impl Into<String>, success: bool) -> Self {
        Self::ToolCallResult(ToolCallResultEvent {
            call_id: call_id.into(),
            tool_name: name.into(),
            success,
            output: None,
            error: None,
            duration_ms: 0,
            timestamp: Utc::now(),
        })
    }

    /// Create an error event
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(ErrorEvent {
            message: message.into(),
            code: None,
            details: None,
            timestamp: Utc::now(),
        })
    }

    /// Create a result event
    pub fn result(content: impl Into<String>) -> Self {
        Self::Result(ResultEvent {
            content: content.into(),
            cost: None,
            duration_ms: 0,
            session_id: None,
            timestamp: Utc::now(),
        })
    }

    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::System(_) => "system",
            Self::Assistant(_) => "assistant",
            Self::ToolCallStart(_) => "tool_call_start",
            Self::ToolCallResult(_) => "tool_call_result",
            Self::UserPrompt(_) => "user_prompt",
            Self::Error(_) => "error",
            Self::Result(_) => "result",
        }
    }

    /// Get timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::System(e) => e.timestamp,
            Self::Assistant(e) => e.timestamp,
            Self::ToolCallStart(e) => e.timestamp,
            Self::ToolCallResult(e) => e.timestamp,
            Self::UserPrompt(e) => e.timestamp,
            Self::Error(e) => e.timestamp,
            Self::Result(e) => e.timestamp,
        }
    }

    /// Serialize to JSON line
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// System event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Assistant response event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantEvent {
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tool call start event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStartEvent {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl ToolCallStartEvent {
    /// Set arguments
    pub fn with_arguments(mut self, args: serde_json::Value) -> Self {
        self.arguments = args;
        self
    }
}

/// Tool call result event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResultEvent {
    pub call_id: String,
    pub tool_name: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl ToolCallResultEvent {
    /// Set output
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Set error
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// User prompt event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptEvent {
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl UserPromptEvent {
    /// Create a new user prompt event
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl ErrorEvent {
    /// Set error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set details
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Result/completion event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultEvent {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostInfo>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl ResultEvent {
    /// Set cost info
    pub fn with_cost(mut self, cost: CostInfo) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Cost information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}

impl CostInfo {
    /// Create new cost info
    pub fn new(input_tokens: usize, output_tokens: usize) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost_usd: None,
        }
    }

    /// Set estimated cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.estimated_cost_usd = Some(cost);
        self
    }
}

/// Final output structure for JSON mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonOutput {
    /// The final result/response
    pub result: String,

    /// Session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Cost information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostInfo>,

    /// Total duration in milliseconds
    pub duration_ms: u64,

    /// Tool calls made
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCallSummary>,

    /// Whether execution was successful
    pub success: bool,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl JsonOutput {
    /// Create a successful output
    pub fn success(result: impl Into<String>) -> Self {
        Self {
            result: result.into(),
            session_id: None,
            cost: None,
            duration_ms: 0,
            tool_calls: Vec::new(),
            success: true,
            error: None,
        }
    }

    /// Create a failed output
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            result: String::new(),
            session_id: None,
            cost: None,
            duration_ms: 0,
            tool_calls: Vec::new(),
            success: false,
            error: Some(error.into()),
        }
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set cost
    pub fn with_cost(mut self, cost: CostInfo) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Add tool call
    pub fn with_tool_call(mut self, call: ToolCallSummary) -> Self {
        self.tool_calls.push(call);
        self
    }
}

/// Summary of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub call_id: String,
    pub tool_name: String,
    pub success: bool,
    pub duration_ms: u64,
}

impl ToolCallSummary {
    /// Create a new summary
    pub fn new(call_id: impl Into<String>, tool_name: impl Into<String>, success: bool) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            success,
            duration_ms: 0,
        }
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!(OutputFormat::from_str("text"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(
            OutputFormat::from_str("stream-json"),
            Some(OutputFormat::StreamJson)
        );
        assert_eq!(
            OutputFormat::from_str("jsonl"),
            Some(OutputFormat::StreamJson)
        );
        assert_eq!(OutputFormat::from_str("invalid"), None);
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Text.to_string(), "text");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::StreamJson.to_string(), "stream-json");
    }

    #[test]
    fn test_output_format_is_json() {
        assert!(!OutputFormat::Text.is_json());
        assert!(OutputFormat::Json.is_json());
        assert!(OutputFormat::StreamJson.is_json());
    }

    #[test]
    fn test_output_event_system() {
        let event = OutputEvent::system("Test message");
        assert_eq!(event.event_type(), "system");
    }

    #[test]
    fn test_output_event_assistant() {
        let event = OutputEvent::assistant("Response content");
        assert_eq!(event.event_type(), "assistant");
    }

    #[test]
    fn test_output_event_tool_start() {
        let event = OutputEvent::tool_start("call_1", "Read");
        assert_eq!(event.event_type(), "tool_call_start");
    }

    #[test]
    fn test_output_event_tool_result() {
        let event = OutputEvent::tool_result("call_1", "Read", true);
        assert_eq!(event.event_type(), "tool_call_result");
    }

    #[test]
    fn test_output_event_to_json_line() {
        let event = OutputEvent::system("Test");
        let json = event.to_json_line();
        assert!(json.contains("system"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_json_output_success() {
        let output = JsonOutput::success("Done")
            .with_session("session-123")
            .with_duration(1000);

        assert!(output.success);
        assert_eq!(output.result, "Done");
        assert_eq!(output.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_json_output_failure() {
        let output = JsonOutput::failure("Error occurred");

        assert!(!output.success);
        assert_eq!(output.error, Some("Error occurred".to_string()));
    }

    #[test]
    fn test_cost_info() {
        let cost = CostInfo::new(100, 50).with_cost(0.001);

        assert_eq!(cost.input_tokens, 100);
        assert_eq!(cost.output_tokens, 50);
        assert_eq!(cost.total_tokens, 150);
        assert_eq!(cost.estimated_cost_usd, Some(0.001));
    }

    #[test]
    fn test_tool_call_summary() {
        let summary = ToolCallSummary::new("call_1", "Read", true).with_duration(100);

        assert_eq!(summary.call_id, "call_1");
        assert!(summary.success);
        assert_eq!(summary.duration_ms, 100);
    }

    #[test]
    fn test_error_event_builder() {
        if let OutputEvent::Error(e) = OutputEvent::error("Test error") {
            let e = ErrorEvent {
                message: e.message,
                code: Some("E001".to_string()),
                details: Some(serde_json::json!({"key": "value"})),
                timestamp: e.timestamp,
            };
            assert_eq!(e.code, Some("E001".to_string()));
        }
    }

    #[test]
    fn test_result_event_builder() {
        let result = ResultEvent {
            content: "Done".to_string(),
            cost: Some(CostInfo::new(10, 20)),
            duration_ms: 500,
            session_id: Some("sess".to_string()),
            timestamp: Utc::now(),
        };

        assert_eq!(result.duration_ms, 500);
        assert!(result.cost.is_some());
    }

    #[test]
    fn test_serialization() {
        let event = OutputEvent::assistant("Hello");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("assistant"));

        let output = JsonOutput::success("Done");
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"success\":true"));
    }
}
