//! JSON editing tool using JSONPath

use async_trait::async_trait;
use jsonpath_rust::JsonPathFinder;
use std::path::PathBuf;
use tokio::fs;
use sage_core::tools::base::{FileSystemTool, Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Tool for editing JSON files using JSONPath
pub struct JsonEditTool {
    working_directory: PathBuf,
}

impl JsonEditTool {
    /// Create a new JSON edit tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create a JSON edit tool with specific working directory
    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
        Self {
            working_directory: working_dir.into(),
        }
    }

    /// Read and parse JSON file
    async fn read_json(&self, file_path: &str) -> Result<serde_json::Value, ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(&path).await.map_err(|e| ToolError::Io(e))?;

        serde_json::from_str(&content).map_err(|e| {
            ToolError::ExecutionFailed(format!("Invalid JSON in file {}: {}", file_path, e))
        })
    }

    /// Write JSON to file
    async fn write_json(
        &self,
        file_path: &str,
        json: &serde_json::Value,
    ) -> Result<(), ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        let content = serde_json::to_string_pretty(json).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to serialize JSON: {}", e))
        })?;

        fs::write(&path, content)
            .await
            .map_err(|e| ToolError::Io(e))?;

        Ok(())
    }

    /// Query JSON using JSONPath
    async fn query_json(
        &self,
        file_path: &str,
        json_path: &str,
    ) -> Result<ToolResult, ToolError> {
        let json = self.read_json(file_path).await?;

        let finder = JsonPathFinder::from_str(&json.to_string(), json_path).map_err(|e| {
            ToolError::InvalidArguments(format!("Invalid JSONPath '{}': {}", json_path, e))
        })?;

        let result = finder.find();

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "JSONPath query result for '{}' in {}:\n{}",
                json_path,
                file_path,
                serde_json::to_string_pretty(&result).unwrap_or_default()
            ),
        ))
    }

    /// Edit JSON using JSONPath
    async fn edit_json(
        &self,
        file_path: &str,
        json_path: &str,
        new_value: &str,
    ) -> Result<ToolResult, ToolError> {
        let mut json = self.read_json(file_path).await?;

        // Parse the new value
        let new_val: serde_json::Value = if new_value.starts_with('"') && new_value.ends_with('"') {
            // String value
            serde_json::Value::String(new_value[1..new_value.len() - 1].to_string())
        } else if new_value == "true" || new_value == "false" {
            // Boolean value
            serde_json::Value::Bool(new_value == "true")
        } else if new_value == "null" {
            // Null value
            serde_json::Value::Null
        } else if let Ok(num) = new_value.parse::<i64>() {
            // Integer value
            serde_json::Value::Number(serde_json::Number::from(num))
        } else if let Ok(num) = new_value.parse::<f64>() {
            // Float value
            serde_json::Value::Number(
                serde_json::Number::from_f64(num).ok_or_else(|| {
                    ToolError::InvalidArguments("Invalid number value".to_string())
                })?,
            )
        } else {
            // Try to parse as JSON
            serde_json::from_str(new_value).map_err(|e| {
                ToolError::InvalidArguments(format!("Invalid JSON value '{}': {}", new_value, e))
            })?
        };

        // Apply the edit using a simple JSONPath implementation
        if json_path == "$" {
            json = new_val;
        } else {
            // For simplicity, we'll handle basic paths like $.key or $.key.subkey
            let path_parts: Vec<&str> = json_path
                .strip_prefix("$.")
                .unwrap_or(json_path)
                .split('.')
                .collect();

            self.set_json_value(&mut json, &path_parts, new_val)?;
        }

        self.write_json(file_path, &json).await?;

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully updated '{}' in {} with value: {}",
                json_path, file_path, new_value
            ),
        ))
    }

    /// Set a value in JSON using path parts
    fn set_json_value(
        &self,
        json: &mut serde_json::Value,
        path_parts: &[&str],
        new_value: serde_json::Value,
    ) -> Result<(), ToolError> {
        if path_parts.is_empty() {
            return Err(ToolError::InvalidArguments("Empty path".to_string()));
        }

        if path_parts.len() == 1 {
            // Base case: set the value
            if let serde_json::Value::Object(map) = json {
                map.insert(path_parts[0].to_string(), new_value);
            } else {
                return Err(ToolError::ExecutionFailed(
                    "Cannot set property on non-object".to_string(),
                ));
            }
        } else {
            // Recursive case: navigate deeper
            if let serde_json::Value::Object(map) = json {
                let key = path_parts[0];
                if !map.contains_key(key) {
                    map.insert(key.to_string(), serde_json::Value::Object(serde_json::Map::new()));
                }
                if let Some(sub_value) = map.get_mut(key) {
                    self.set_json_value(sub_value, &path_parts[1..], new_value)?;
                }
            } else {
                return Err(ToolError::ExecutionFailed(
                    "Cannot navigate into non-object".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for JsonEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for JsonEditTool {
    fn name(&self) -> &str {
        "json_edit_tool"
    }

    fn description(&self) -> &str {
        "Edit JSON files using JSONPath queries. Can read, query, and modify JSON data."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("command", "The command to execute: 'read', 'query', or 'edit'"),
                ToolParameter::string("path", "Path to the JSON file"),
                ToolParameter::optional_string("json_path", "JSONPath expression (for query and edit commands)"),
                ToolParameter::optional_string("new_value", "New value to set (for edit command)"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call
            .get_string("command")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

        let path = call
            .get_string("path")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        let mut result = match command.as_str() {
            "read" => {
                let json = self.read_json(&path).await?;
                ToolResult::success(
                    "",
                    self.name(),
                    format!(
                        "Content of {}:\n{}",
                        path,
                        serde_json::to_string_pretty(&json).unwrap_or_default()
                    ),
                )
            }
            "query" => {
                let json_path = call.get_string("json_path").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'json_path' parameter for query".to_string())
                })?;
                self.query_json(&path, &json_path).await?
            }
            "edit" => {
                let json_path = call.get_string("json_path").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'json_path' parameter for edit".to_string())
                })?;
                let new_value = call.get_string("new_value").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'new_value' parameter for edit".to_string())
                })?;
                self.edit_json(&path, &json_path, &new_value).await?
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown command: {}. Use 'read', 'query', or 'edit'",
                    command
                )));
            }
        };

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let command = call
            .get_string("command")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;

        let _path = call
            .get_string("path")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        match command.as_str() {
            "read" => {
                // No additional parameters needed
            }
            "query" => {
                if call.get_string("json_path").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'json_path' parameter for query".to_string(),
                    ));
                }
            }
            "edit" => {
                if call.get_string("json_path").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'json_path' parameter for edit".to_string(),
                    ));
                }
                if call.get_string("new_value").is_none() {
                    return Err(ToolError::InvalidArguments(
                        "Missing 'new_value' parameter for edit".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown command: {}. Use 'read', 'query', or 'edit'",
                    command
                )));
            }
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(30) // 30 seconds
    }

    fn supports_parallel_execution(&self) -> bool {
        false // File operations should be sequential
    }
}

impl FileSystemTool for JsonEditTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }
}
