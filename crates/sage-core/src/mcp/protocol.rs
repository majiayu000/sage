//! MCP protocol message types
//!
//! Implements the JSON-RPC based MCP protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Protocol version constant
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// JSON-RPC version
pub const JSONRPC_VERSION: &str = "2.0";

/// MCP message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpMessage {
    /// Request message
    Request(McpRequest),
    /// Response message
    Response(McpResponse),
    /// Notification message (no id)
    Notification(McpNotification),
}

impl McpMessage {
    /// Check if this is a response
    pub fn is_response(&self) -> bool {
        matches!(self, Self::Response(_))
    }

    /// Check if this is a request
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    /// Check if this is a notification
    pub fn is_notification(&self) -> bool {
        matches!(self, Self::Notification(_))
    }

    /// Get the message ID if present
    pub fn id(&self) -> Option<&RequestId> {
        match self {
            Self::Request(req) => Some(&req.id),
            Self::Response(res) => Some(&res.id),
            Self::Notification(_) => None,
        }
    }
}

/// Request ID (can be string or number)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// String ID
    String(String),
    /// Number ID
    Number(i64),
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => write!(f, "{}", n),
        }
    }
}

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Method name
    pub method: String,
    /// Optional parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl McpRequest {
    /// Create a new request
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            method: method.into(),
            params: None,
        }
    }

    /// Add parameters to the request
    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID this response corresponds to
    pub id: RequestId,
    /// Result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpRpcError>,
}

impl McpResponse {
    /// Create a success response
    pub fn success(id: impl Into<RequestId>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: impl Into<RequestId>, error: McpRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }

    /// Check if this is a success response
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Get the result, consuming the response
    pub fn into_result(self) -> Result<Value, McpRpcError> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(self.result.unwrap_or(Value::Null)),
        }
    }
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl McpRpcError {
    /// Create a new error
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Add data to the error
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    // Standard JSON-RPC error codes

    /// Parse error (-32700)
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    /// Invalid request (-32600)
    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid request")
    }

    /// Method not found (-32601)
    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    /// Invalid params (-32602)
    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params")
    }

    /// Internal error (-32603)
    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }
}

impl std::fmt::Display for McpRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for McpRpcError {}

/// JSON-RPC notification (no id, no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNotification {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Optional parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl McpNotification {
    /// Create a new notification
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params: None,
        }
    }

    /// Add parameters
    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// MCP method names
pub mod methods {
    /// Initialize
    pub const INITIALIZE: &str = "initialize";
    /// Initialized notification
    pub const INITIALIZED: &str = "notifications/initialized";

    /// List tools
    pub const TOOLS_LIST: &str = "tools/list";
    /// Call tool
    pub const TOOLS_CALL: &str = "tools/call";

    /// List resources
    pub const RESOURCES_LIST: &str = "resources/list";
    /// Read resource
    pub const RESOURCES_READ: &str = "resources/read";
    /// Subscribe to resource
    pub const RESOURCES_SUBSCRIBE: &str = "resources/subscribe";
    /// Unsubscribe from resource
    pub const RESOURCES_UNSUBSCRIBE: &str = "resources/unsubscribe";

    /// List prompts
    pub const PROMPTS_LIST: &str = "prompts/list";
    /// Get prompt
    pub const PROMPTS_GET: &str = "prompts/get";

    /// Ping
    pub const PING: &str = "ping";

    /// Cancellation notification
    pub const CANCELLED: &str = "notifications/cancelled";

    /// Progress notification
    pub const PROGRESS: &str = "notifications/progress";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = McpRequest::new(1i64, "tools/list");
        let json = serde_json::to_string(&req).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"tools/list\""));
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn test_request_with_params() {
        let req = McpRequest::new("req-1", "tools/call").with_params(serde_json::json!({
            "name": "read_file",
            "arguments": {"path": "/tmp/test.txt"}
        }));

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("read_file"));
    }

    #[test]
    fn test_response_success() {
        let res = McpResponse::success(1i64, serde_json::json!({"status": "ok"}));

        assert!(res.is_success());
        let result = res.into_result().unwrap();
        assert_eq!(result["status"], "ok");
    }

    #[test]
    fn test_response_error() {
        let res = McpResponse::error(1i64, McpRpcError::method_not_found());

        assert!(!res.is_success());
        let err = res.into_result().unwrap_err();
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn test_notification() {
        let notif = McpNotification::new("notifications/initialized");
        let json = serde_json::to_string(&notif).unwrap();

        assert!(!json.contains("\"id\""));
        assert!(json.contains("notifications/initialized"));
    }

    #[test]
    fn test_parse_message() {
        // Parse request
        let req_json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let msg: McpMessage = serde_json::from_str(req_json).unwrap();
        assert!(msg.is_request());

        // Parse response
        let res_json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let msg: McpMessage = serde_json::from_str(res_json).unwrap();
        assert!(msg.is_response());
    }
}
