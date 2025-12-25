//! JSON output formatting types

use serde::{Deserialize, Serialize};

use super::base::{CostInfo, ToolCallSummary};

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
