//! Output formatting system
//!
//! This module provides structured output formatting for different output modes:
//! - `text`: Human-readable text output (default)
//! - `json`: Structured JSON output
//! - `stream-json`: JSONL streaming output (one JSON per line)
//!
//! # Overview
//!
//! The output system enables tool integration and programmatic consumption
//! of Sage Agent's output through structured formats.
//!
//! # Output Formats
//!
//! ## Text Format
//! Human-readable output with optional colors and timestamps.
//!
//! ## JSON Format
//! Structured JSON output with all information in a single object.
//!
//! ## Stream JSON Format
//! JSONL format where each event is a separate JSON line:
//! ```json
//! {"type":"system","message":"Starting...","timestamp":"..."}
//! {"type":"tool_call_start","tool_name":"Read","call_id":"..."}
//! {"type":"tool_call_result","tool_name":"Read","success":true}
//! {"type":"assistant","content":"Here's what I found..."}
//! {"type":"result","content":"Done","success":true}
//! ```
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use sage_core::output::{OutputFormat, OutputWriter, OutputEvent};
//! use std::io::stdout;
//!
//! // Create writer for stream-json format
//! let mut writer = OutputWriter::new(stdout(), OutputFormat::StreamJson);
//!
//! // Write events as they occur
//! writer.write_event(&OutputEvent::system("Starting..."))?;
//! writer.write_event(&OutputEvent::tool_start("call_1", "Read"))?;
//! writer.write_event(&OutputEvent::tool_result("call_1", "Read", true))?;
//!
//! // Write final result
//! let output = JsonOutput::success("Task complete");
//! writer.write_final(&output)?;
//! ```
//!
//! # Event Types
//!
//! | Event | Description |
//! |-------|-------------|
//! | `system` | System messages |
//! | `assistant` | Assistant responses |
//! | `tool_call_start` | Tool execution started |
//! | `tool_call_result` | Tool execution completed |
//! | `user_prompt` | User input received |
//! | `error` | Error occurred |
//! | `result` | Final result |

pub mod formatter;
pub mod types;

pub use formatter::{
    JsonFormatter, OutputFormatter, OutputWriter, StreamJsonFormatter, TextFormatter,
    create_formatter,
};
pub use types::{
    AssistantEvent, CostInfo, ErrorEvent, JsonOutput, OutputEvent, OutputFormat, ResultEvent,
    SystemEvent, ToolCallResultEvent, ToolCallStartEvent, ToolCallSummary, UserPromptEvent,
};
