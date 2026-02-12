//! Tool call, result, schema, and parameter types shared across tools, agent, llm, and other modules

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::tool_error::ToolError;

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

        // Try camelCase version (e.g., "file_path" -> "filePath")
        let camel = to_camel_case(key);
        if camel != key {
            if let Some(value) = self.arguments.get(&camel) {
                if let Ok(result) = serde_json::from_value(value.clone()) {
                    return Some(result);
                }
            }
        }

        // Try snake_case version (e.g., "filePath" -> "file_path")
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

    /// Require a string argument, returning error if missing
    ///
    /// Use this instead of `get_string().ok_or_else(...)` to reduce boilerplate.
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

/// Convert snake_case to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert camelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
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

    /// Create a flexible tool schema with custom parameters JSON
    /// Useful for platform tools that accept arbitrary parameters
    pub fn new_flexible<S: Into<String>>(
        name: S,
        description: S,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("file_path"), "filePath");
        assert_eq!(to_camel_case("working_directory"), "workingDirectory");
        assert_eq!(to_camel_case("simple"), "simple");
        assert_eq!(to_camel_case("a_b_c"), "aBC");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("filePath"), "file_path");
        assert_eq!(to_snake_case("workingDirectory"), "working_directory");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case("ABC"), "a_b_c");
    }

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
}
