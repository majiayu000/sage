//! Tool call, result, schema, and parameter types shared across tools, agent, llm, and other modules

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::case_conversion::{to_camel_case, to_snake_case};
use super::tool_error::ToolError;

// Re-export schema types
pub use super::schema::{ToolParameter, ToolSchema};

/// A tool call from the LLM
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments to pass to the tool
    pub arguments: HashMap<String, serde_json::Value>,
    /// Optional call ID for tracking
    pub call_id: Option<String>,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new<S: Into<String>>(
        id: S,
        name: S,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
            call_id: None,
        }
    }

    /// Create a tool call with call ID
    pub fn with_call_id<S: Into<String>>(mut self, call_id: S) -> Self {
        self.call_id = Some(call_id.into());
        self
    }

    /// Get a typed argument value, trying both snake_case and camelCase
    pub fn get_argument<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Try exact key first
        if let Some(value) = self.arguments.get(key) {
            if let Ok(result) = serde_json::from_value(value.clone()) {
                return Some(result);
            }
        }

        // Try camelCase version
        let camel = to_camel_case(key);
        if camel != key {
            if let Some(value) = self.arguments.get(&camel) {
                if let Ok(result) = serde_json::from_value(value.clone()) {
                    return Some(result);
                }
            }
        }

        // Try snake_case version
        let snake = to_snake_case(key);
        if snake != key {
            if let Some(value) = self.arguments.get(&snake) {
                if let Ok(result) = serde_json::from_value(value.clone()) {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Get a string argument
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get_argument::<String>(key)
    }

    /// Get a boolean argument
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_argument::<bool>(key)
    }

    /// Get a number argument
    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.get_argument::<f64>(key)
    }

    /// Get an integer argument
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get_argument::<i64>(key)
    }

    /// Get a number argument as usize, safely handling NaN/Infinity/negative values.
    pub fn get_usize(&self, key: &str, default: usize) -> usize {
        match self.get_number(key) {
            Some(n) if n.is_finite() && n >= 0.0 => (n as u64) as usize,
            Some(_) => default,
            None => default,
        }
    }

    /// Get a number argument as u32, safely handling NaN/Infinity/negative values.
    pub fn get_u32(&self, key: &str, default: u32) -> u32 {
        match self.get_number(key) {
            Some(n) if n.is_finite() && n >= 0.0 => {
                let clamped = n.min(u32::MAX as f64);
                clamped as u32
            }
            Some(_) => default,
            None => default,
        }
    }

    /// Require a string argument, returning error if missing
    pub fn require_string(&self, key: &str) -> Result<String, ToolError> {
        self.get_string(key).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Missing required parameter '{}'", key))
        })
    }

    /// Require a path argument, returning error if missing
    pub fn require_path(&self, key: &str) -> Result<PathBuf, ToolError> {
        self.get_string(key).map(PathBuf::from).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Missing required parameter '{}'", key))
        })
    }

    /// Require a boolean argument, returning error if missing
    pub fn require_bool(&self, key: &str) -> Result<bool, ToolError> {
        self.get_bool(key).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Missing required parameter '{}'", key))
        })
    }

    /// Require a number argument, returning error if missing
    pub fn require_number(&self, key: &str) -> Result<f64, ToolError> {
        self.get_number(key).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Missing required parameter '{}'", key))
        })
    }

    /// Require an integer argument, returning error if missing
    pub fn require_i64(&self, key: &str) -> Result<i64, ToolError> {
        self.get_i64(key).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Missing required parameter '{}'", key))
        })
    }

    /// Get a typed argument, returning error if missing or wrong type
    pub fn require_argument<T>(&self, key: &str) -> Result<T, ToolError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.get_argument::<T>(key).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Missing or invalid parameter '{}'", key))
        })
    }
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool call ID this result corresponds to
    pub call_id: String,
    /// Name of the tool that was executed
    pub tool_name: String,
    /// Whether the tool execution was successful
    pub success: bool,
    /// Output from the tool (if successful)
    pub output: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Exit code (for command-line tools)
    pub exit_code: Option<i32>,
    /// Execution time in milliseconds
    pub execution_time_ms: Option<u64>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolResult {
    /// Create a successful tool result
    pub fn success(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            success: true,
            output: Some(output.into()),
            error: None,
            exit_code: Some(0),
            execution_time_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed tool result
    pub fn error(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            success: false,
            output: None,
            error: Some(error.into()),
            exit_code: Some(1),
            execution_time_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Add execution time
    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.execution_time_ms = Some(time_ms);
        self
    }

    /// Add metadata
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_argument_with_case_fallback() {
        let mut args = HashMap::new();
        args.insert("filePath".to_string(), serde_json::json!("/test/path"));
        args.insert("content".to_string(), serde_json::json!("hello"));

        let call = ToolCall::new("1", "Write", args);

        assert_eq!(call.get_string("file_path"), Some("/test/path".to_string()));
        assert_eq!(call.get_string("content"), Some("hello".to_string()));
    }

    #[test]
    fn test_get_argument_snake_to_camel() {
        let mut args = HashMap::new();
        args.insert("file_path".to_string(), serde_json::json!("/test/path"));

        let call = ToolCall::new("1", "Write", args);

        assert_eq!(call.get_string("filePath"), Some("/test/path".to_string()));
    }

    #[test]
    fn test_get_usize_safe_conversion() {
        let mut args = HashMap::new();
        args.insert("normal".to_string(), serde_json::json!(42.0));
        args.insert("negative".to_string(), serde_json::json!(-5.0));
        args.insert("zero".to_string(), serde_json::json!(0.0));
        args.insert("fractional".to_string(), serde_json::json!(3.7));

        let call = ToolCall::new("1", "test", args);

        assert_eq!(call.get_usize("normal", 0), 42);
        assert_eq!(call.get_usize("negative", 99), 99);
        assert_eq!(call.get_usize("zero", 99), 0);
        assert_eq!(call.get_usize("fractional", 0), 3);
        assert_eq!(call.get_usize("missing", 7), 7);
    }

    #[test]
    fn test_get_u32_safe_conversion() {
        let mut args = HashMap::new();
        args.insert("normal".to_string(), serde_json::json!(100.0));
        args.insert("large".to_string(), serde_json::json!(5_000_000_000.0));
        args.insert("negative".to_string(), serde_json::json!(-1.0));

        let call = ToolCall::new("1", "test", args);

        assert_eq!(call.get_u32("normal", 0), 100);
        assert_eq!(call.get_u32("large", 0), u32::MAX);
        assert_eq!(call.get_u32("negative", 5), 5);
        assert_eq!(call.get_u32("missing", 1), 1);
    }
}
