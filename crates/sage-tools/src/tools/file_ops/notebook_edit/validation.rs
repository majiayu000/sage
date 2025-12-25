//! Validation logic for notebook edit tool calls

use sage_core::tools::base::ToolError;
use sage_core::tools::types::ToolCall;

/// Validate a notebook edit tool call
pub fn validate_call(call: &ToolCall) -> Result<(), ToolError> {
    let _notebook_path = call.get_string("notebook_path").ok_or_else(|| {
        ToolError::InvalidArguments("Missing 'notebook_path' parameter".to_string())
    })?;

    let edit_mode = call
        .get_string("edit_mode")
        .unwrap_or_else(|| "replace".to_string());

    match edit_mode.as_str() {
        "replace" => {
            if call.get_string("new_source").is_none() {
                return Err(ToolError::InvalidArguments(
                    "Missing 'new_source' parameter for replace".to_string(),
                ));
            }
            if call.get_string("cell_id").is_none() {
                return Err(ToolError::InvalidArguments(
                    "Missing 'cell_id' parameter for replace".to_string(),
                ));
            }
        }
        "insert" => {
            if call.get_string("new_source").is_none() {
                return Err(ToolError::InvalidArguments(
                    "Missing 'new_source' parameter for insert".to_string(),
                ));
            }
            if call.get_string("cell_type").is_none() {
                return Err(ToolError::InvalidArguments(
                    "Missing 'cell_type' parameter for insert".to_string(),
                ));
            }
        }
        "delete" => {
            if call.get_string("cell_id").is_none() {
                return Err(ToolError::InvalidArguments(
                    "Missing 'cell_id' parameter for delete".to_string(),
                ));
            }
        }
        _ => {
            return Err(ToolError::InvalidArguments(format!(
                "Unknown edit_mode: {}. Use 'replace', 'insert', or 'delete'",
                edit_mode
            )));
        }
    }

    Ok(())
}
