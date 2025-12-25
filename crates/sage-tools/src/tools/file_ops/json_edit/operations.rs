//! JSON file operations (read, write, query, edit)

use super::types::JsonEditTool;
use jsonpath_rust::JsonPathFinder;
use sage_core::tools::ToolError;
use sage_core::tools::base::FileSystemTool;
use sage_core::tools::types::ToolResult;
use tokio::fs;

impl JsonEditTool {
    /// Read and parse JSON file
    pub(super) async fn read_json(&self, file_path: &str) -> Result<serde_json::Value, ToolError> {
        let path = self.resolve_path(file_path);

        // Security check
        if !self.is_safe_path(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(&path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to read JSON file '{}': {}", file_path, e))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            ToolError::ExecutionFailed(format!("Invalid JSON in file {}: {}", file_path, e))
        })
    }

    /// Write JSON to file
    pub(super) async fn write_json(
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

        let content = serde_json::to_string_pretty(json)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize JSON: {}", e)))?;

        fs::write(&path, content).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to write JSON file '{}': {}", file_path, e))
        })?;

        Ok(())
    }

    /// Query JSON using JSONPath
    pub(super) async fn query_json(
        &self,
        file_path: &str,
        json_path: &str,
    ) -> Result<ToolResult, ToolError> {
        let json = self.read_json(file_path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read JSON for query from '{}': {}",
                file_path, e
            ))
        })?;

        let finder = JsonPathFinder::from_str(&json.to_string(), json_path).map_err(|e| {
            ToolError::InvalidArguments(format!("Invalid JSONPath '{}': {}", json_path, e))
        })?;

        let result = finder.find();

        Ok(ToolResult::success(
            "",
            "json_edit_tool",
            format!(
                "JSONPath query result for '{}' in {}:\n{}",
                json_path,
                file_path,
                serde_json::to_string_pretty(&result).unwrap_or_default()
            ),
        ))
    }

    /// Edit JSON using JSONPath
    pub(super) async fn edit_json(
        &self,
        file_path: &str,
        json_path: &str,
        new_value: &str,
    ) -> Result<ToolResult, ToolError> {
        let mut json = self.read_json(file_path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to read JSON for editing from '{}': {}",
                file_path, e
            ))
        })?;

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

        self.write_json(file_path, &json).await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to write edited JSON to '{}': {}",
                file_path, e
            ))
        })?;

        Ok(ToolResult::success(
            "",
            "json_edit_tool",
            format!(
                "Successfully updated '{}' in {} with value: {}",
                json_path, file_path, new_value
            ),
        ))
    }

    /// Set a value in JSON using path parts
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn set_json_value(
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
                    map.insert(
                        key.to_string(),
                        serde_json::Value::Object(serde_json::Map::new()),
                    );
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
