//! Validation logic for multi-edit operations

use sage_core::tools::base::ToolError;
use sage_core::tools::types::ToolCall;

use super::types::EditOperation;

/// Parse and validate edit operations from a tool call
pub fn parse_edits(call: &ToolCall) -> Result<Vec<EditOperation>, ToolError> {
    let edits_value = call
        .arguments
        .get("edits")
        .ok_or_else(|| ToolError::InvalidArguments("Missing 'edits' parameter".to_string()))?;

    // Try to parse as array of edit operations
    let edits: Vec<EditOperation> = serde_json::from_value(edits_value.clone())
        .map_err(|e| ToolError::InvalidArguments(format!(
            "Invalid 'edits' format: {}. Expected array of {{old_string, new_string, replace_all?}} objects",
            e
        )))?;

    if edits.is_empty() {
        return Err(ToolError::InvalidArguments(
            "The 'edits' array must contain at least one edit operation".to_string(),
        ));
    }

    // Validate each edit
    for (i, edit) in edits.iter().enumerate() {
        if edit.old_string.is_empty() {
            return Err(ToolError::InvalidArguments(format!(
                "Edit {} has empty 'old_string'. Cannot replace empty strings",
                i + 1
            )));
        }
        if edit.old_string == edit.new_string {
            return Err(ToolError::InvalidArguments(format!(
                "Edit {} has identical 'old_string' and 'new_string'. No change would be made",
                i + 1
            )));
        }
    }

    Ok(edits)
}
