use crate::tools::types::ToolCall;
use std::collections::HashMap;
use std::path::Path;

pub(super) fn workspace_dir() -> &'static Path {
    Path::new("/workspace/sage")
}

pub(super) fn read_call(path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "file_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", "read", arguments)
}

pub(super) fn path_call(tool_name: &str, path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", tool_name, arguments)
}

pub(super) fn grep_call_without_path(pattern: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "pattern".to_string(),
        serde_json::Value::String(pattern.to_string()),
    );
    ToolCall::new("call-1", "grep", arguments)
}

pub(super) fn glob_call(pattern: &str, path: Option<&str>) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "pattern".to_string(),
        serde_json::Value::String(pattern.to_string()),
    );
    if let Some(path) = path {
        arguments.insert(
            "path".to_string(),
            serde_json::Value::String(path.to_string()),
        );
    }
    ToolCall::new("call-1", "glob", arguments)
}

pub(super) fn notebook_call(path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "notebook_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", "notebook_edit", arguments)
}
