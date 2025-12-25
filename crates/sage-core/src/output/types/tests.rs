//! Tests for output types

use super::*;
use chrono::Utc;

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
