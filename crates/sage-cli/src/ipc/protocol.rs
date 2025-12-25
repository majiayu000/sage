//! IPC Protocol definitions for Node.js ↔ Rust communication
//!
//! Uses JSON-Lines format for bidirectional streaming communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request from Node.js UI to Rust backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    /// The method name
    pub method: String,
    /// Optional parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

impl IpcRequest {
    /// Parse request from JSON line
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim())
    }

    /// Get the method name
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Parse params as ChatParams
    pub fn as_chat_params(&self) -> Option<ChatParams> {
        serde_json::from_value(self.params.clone()).ok()
    }

    /// Parse params as a struct with config_file field
    pub fn get_config_file(&self) -> Option<String> {
        self.params
            .get("config_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Parse params as a struct with task_id field
    pub fn get_task_id(&self) -> Option<String> {
        self.params
            .get("task_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

/// Parameters for chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatParams {
    pub message: String,
    #[serde(default)]
    pub config_file: Option<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Unique request ID for correlation
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Response/Event from Rust backend to Node.js UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcEvent {
    /// Backend is ready
    #[serde(rename = "ready")]
    Ready { version: String },

    /// Pong response to ping
    #[serde(rename = "pong")]
    Pong,

    /// Request acknowledged
    #[serde(rename = "ack")]
    Ack { request_id: String },

    /// Tool execution started
    #[serde(rename = "tool_started")]
    ToolStarted {
        request_id: String,
        tool_id: String,
        tool_name: String,
        #[serde(default)]
        args: Option<serde_json::Value>,
    },

    /// Tool execution progress (streaming output)
    #[serde(rename = "tool_progress")]
    ToolProgress {
        request_id: String,
        tool_id: String,
        output: String,
    },

    /// Tool execution completed
    #[serde(rename = "tool_completed")]
    ToolCompleted {
        request_id: String,
        tool_id: String,
        success: bool,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        error: Option<String>,
        duration_ms: u64,
    },

    /// LLM is thinking
    #[serde(rename = "llm_thinking")]
    LlmThinking { request_id: String },

    /// LLM streaming response chunk
    #[serde(rename = "llm_chunk")]
    LlmChunk { request_id: String, content: String },

    /// LLM response completed
    #[serde(rename = "llm_done")]
    LlmDone {
        request_id: String,
        content: String,
        #[serde(default)]
        tool_calls: Vec<ToolCallInfo>,
    },

    /// Chat request completed (final response)
    #[serde(rename = "chat_completed")]
    ChatCompleted {
        request_id: String,
        content: String,
        completed: bool,
        #[serde(default)]
        tool_results: Vec<ToolResultInfo>,
        duration_ms: u64,
    },

    /// Error occurred
    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        request_id: Option<String>,
        code: String,
        message: String,
    },

    /// Configuration info
    #[serde(rename = "config")]
    Config(ConfigInfo),

    /// List of available tools
    #[serde(rename = "tools")]
    Tools { tools: Vec<ToolInfo> },

    /// Shutdown acknowledged
    #[serde(rename = "shutdown_ack")]
    ShutdownAck,
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub args: HashMap<String, serde_json::Value>,
}

/// Tool result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub tool_id: String,
    pub tool_name: String,
    pub success: bool,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Configuration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub provider: String,
    pub model: String,
    pub max_steps: Option<u32>,
    pub working_directory: String,
    pub total_token_budget: Option<u64>,
}

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<ToolParameter>,
}

/// Tool parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub param_type: String,
}

impl IpcEvent {
    /// Serialize event to JSON line (with newline)
    pub fn to_json_line(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"type":"error","code":"serialize_error","message":"{}"}}"#, e)
        });
        format!("{}\n", json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_parsing() {
        let json = r#"{"method":"chat","params":{"message":"test","request_id":"r1"}}"#;
        let request = IpcRequest::from_json_line(json).unwrap();

        assert_eq!(request.method(), "chat");
        let params = request.as_chat_params().unwrap();
        assert_eq!(params.message, "test");
        assert_eq!(params.request_id, Some("r1".to_string()));
    }

    #[test]
    fn test_ping_request() {
        let json = r#"{"method":"ping","params":{}}"#;
        let request = IpcRequest::from_json_line(json).unwrap();
        assert_eq!(request.method(), "ping");
    }

    #[test]
    fn test_event_serialization() {
        let event = IpcEvent::ToolStarted {
            request_id: "req-1".to_string(),
            tool_id: "tool-1".to_string(),
            tool_name: "Bash".to_string(),
            args: Some(serde_json::json!({"command": "ls"})),
        };

        let line = event.to_json_line();
        assert!(line.ends_with('\n'));
        assert!(line.contains("tool_started"));
    }
}
