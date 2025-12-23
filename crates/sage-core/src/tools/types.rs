//! Tool-related type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    /// Get a typed argument value
    pub fn get_argument<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.arguments
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
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
}

/// Result of a tool execution
///
/// This is the standardized response format for all tools in Sage Agent.
/// Tools SHOULD use the helper methods `ToolResult::success()` and `ToolResult::error()`
/// rather than manually constructing this struct.
///
/// # Standard Format
///
/// All tool responses follow this structure:
/// - `success`: bool - Whether the tool execution succeeded
/// - `output`: Option<String> - The primary output text (present on success)
/// - `error`: Option<String> - Error message (present on failure)
/// - `metadata`: HashMap<String, serde_json::Value> - Additional structured data
/// - `exit_code`: Option<i32> - Exit code for command-line tools (0 = success)
/// - `execution_time_ms`: Option<u64> - Execution duration in milliseconds
///
/// # Examples
///
/// ```rust
/// use sage_core::tools::types::ToolResult;
///
/// // Success case
/// let result = ToolResult::success("call-1", "ReadTool", "File contents here")
///     .with_metadata("lines_read", 42)
///     .with_execution_time(123);
///
/// // Error case
/// let result = ToolResult::error("call-1", "ReadTool", "File not found");
/// ```
///
/// # Best Practices
///
/// 1. Always use `ToolResult::success()` for successful operations
/// 2. Always use `ToolResult::error()` for failed operations
/// 3. Use `.with_metadata()` to add structured data (counts, timestamps, etc.)
/// 4. Use `.with_execution_time()` to track performance
/// 5. Keep `output` as human-readable text, use `metadata` for structured data
/// 6. Set `call_id` from the incoming `ToolCall.id` after constructing the result
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

/// Parameter definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Parameter type (string, number, boolean, object, array)
    pub param_type: String,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value (if any)
    pub default: Option<serde_json::Value>,
    /// Enum values (if applicable)
    pub enum_values: Option<Vec<serde_json::Value>>,
    /// Additional schema properties
    pub properties: HashMap<String, serde_json::Value>,
}

impl ToolParameter {
    /// Create a required string parameter
    pub fn string<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required: true,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Create an optional string parameter
    pub fn optional_string<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required: false,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Create a boolean parameter
    pub fn boolean<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "boolean".to_string(),
            required: true,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Create a number parameter
    pub fn number<S: Into<String>>(name: S, description: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "number".to_string(),
            required: true,
            default: None,
            enum_values: None,
            properties: HashMap::new(),
        }
    }

    /// Make parameter optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set default value
    pub fn with_default<V: Into<serde_json::Value>>(mut self, default: V) -> Self {
        self.default = Some(default.into());
        self
    }
}

/// JSON schema for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input parameters schema
    pub parameters: serde_json::Value,
}

impl ToolSchema {
    /// Create a new tool schema
    pub fn new<S: Into<String>>(name: S, description: S, parameters: Vec<ToolParameter>) -> Self {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in parameters {
            if param.required {
                required.push(param.name.clone());
            }

            let mut param_schema = serde_json::Map::new();
            param_schema.insert("type".to_string(), param.param_type.into());
            param_schema.insert("description".to_string(), param.description.into());

            if let Some(default) = param.default {
                param_schema.insert("default".to_string(), default);
            }

            if let Some(enum_values) = param.enum_values {
                param_schema.insert("enum".to_string(), enum_values.into());
            }

            for (key, value) in param.properties {
                param_schema.insert(key, value);
            }

            properties.insert(param.name, param_schema.into());
        }

        let parameters_schema = serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        });

        Self {
            name: name.into(),
            description: description.into(),
            parameters: parameters_schema,
        }
    }
}
