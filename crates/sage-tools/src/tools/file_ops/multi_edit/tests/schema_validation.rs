//! Schema and validation tests

use crate::tools::file_ops::multi_edit::MultiEditTool;
use sage_core::tools::base::Tool;
use serde_json::json;

use super::common::create_tool_call;

#[test]
fn test_multi_edit_schema() {
    let tool = MultiEditTool::new();
    let schema = tool.schema();
    assert_eq!(schema.name, "MultiEdit");
    assert!(!schema.description.is_empty());
}

#[test]
fn test_multi_edit_validation() {
    let tool = MultiEditTool::new();

    // Valid call
    let call = create_tool_call(
        "test-14",
        "MultiEdit",
        json!({
            "file_path": "/path/to/file.txt",
            "edits": [
                {"old_string": "test", "new_string": "replacement"}
            ]
        }),
    );
    assert!(tool.validate(&call).is_ok());

    // Invalid - missing file_path
    let call = create_tool_call(
        "test-15",
        "MultiEdit",
        json!({
            "edits": [
                {"old_string": "test", "new_string": "replacement"}
            ]
        }),
    );
    assert!(tool.validate(&call).is_err());

    // Invalid - missing edits
    let call = create_tool_call(
        "test-16",
        "MultiEdit",
        json!({
            "file_path": "/path/to/file.txt"
        }),
    );
    assert!(tool.validate(&call).is_err());
}
