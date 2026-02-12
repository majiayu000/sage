//! Wire-format tool call and result types for session persistence.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool call (wire format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolCall {
    /// Tool call ID
    pub id: String,

    /// Tool name
    pub name: String,

    /// Tool arguments
    pub arguments: Value,
}

/// Convert from the canonical `ToolCall` to the wire-format `UnifiedToolCall`.
impl From<&crate::types::ToolCall> for UnifiedToolCall {
    fn from(call: &crate::types::ToolCall) -> Self {
        Self {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: serde_json::to_value(&call.arguments).unwrap_or_default(),
        }
    }
}

/// Tool result (wire format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolResult {
    /// Tool call ID this result is for
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,

    /// Tool name
    #[serde(rename = "toolName")]
    pub tool_name: String,

    /// Result content
    pub content: String,

    /// Whether execution succeeded
    pub success: bool,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl UnifiedToolResult {
    /// Create a successful tool result
    pub fn success(
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            content: content.into(),
            success: true,
            error: None,
        }
    }

    /// Create a failed tool result
    pub fn failure(
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            content: String::new(),
            success: false,
            error: Some(error.into()),
        }
    }
}

/// Convert from the canonical `ToolResult` to the wire-format `UnifiedToolResult`.
impl From<crate::tools::types::ToolResult> for UnifiedToolResult {
    fn from(result: crate::tools::types::ToolResult) -> Self {
        Self {
            tool_call_id: result.call_id,
            tool_name: result.tool_name,
            content: result.output.unwrap_or_default(),
            success: result.success,
            error: result.error,
        }
    }
}

/// Convert from a reference to the canonical `ToolResult`.
impl From<&crate::tools::types::ToolResult> for UnifiedToolResult {
    fn from(result: &crate::tools::types::ToolResult) -> Self {
        Self {
            tool_call_id: result.call_id.clone(),
            tool_name: result.tool_name.clone(),
            content: result.output.clone().unwrap_or_default(),
            success: result.success,
            error: result.error.clone(),
        }
    }
}
