//! Tool trait implementation and schema

use super::types::JsonEditTool;
use async_trait::async_trait;
use sage_core::tools::base::Tool;
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use sage_core::tools::ToolError;

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
                ToolParameter::string(
                    "command",
                    "The command to execute: 'read', 'query', or 'edit'",
                ),
                ToolParameter::string("path", "Path to the JSON file"),
                ToolParameter::optional_string(
                    "json_path",
                    "JSONPath expression (for query and edit commands)",
                ),
                ToolParameter::optional_string("new_value", "New value to set (for edit command)"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        let path = call
            .get_string("path")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;

        let mut result = match command.as_str() {
            "read" => {
                let json = self.read_json(&path).await.map_err(|e| {
                    ToolError::ExecutionFailed(format!(
                        "Failed to read JSON file '{}': {}",
                        path, e
                    ))
                })?;
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
                    ToolError::InvalidArguments(
                        "Missing 'json_path' parameter for query".to_string(),
                    )
                })?;
                self.query_json(&path, &json_path).await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to query JSON at '{}': {}", path, e))
                })?
            }
            "edit" => {
                let json_path = call.get_string("json_path").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'json_path' parameter for edit".to_string(),
                    )
                })?;
                let new_value = call.get_string("new_value").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'new_value' parameter for edit".to_string(),
                    )
                })?;
                self.edit_json(&path, &json_path, &new_value)
                    .await
                    .map_err(|e| {
                        ToolError::ExecutionFailed(format!(
                            "Failed to edit JSON at '{}': {}",
                            path, e
                        ))
                    })?
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
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

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
