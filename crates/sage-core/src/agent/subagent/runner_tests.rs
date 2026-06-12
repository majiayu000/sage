use super::runner::{StepResult, SubAgentRunner};
use crate::tools::types::ToolCall;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn write_settings(temp_dir: &TempDir, content: &str) {
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir).expect("create .sage dir");
    fs::write(sage_dir.join("settings.json"), content).expect("write settings");
}

fn tool_call(name: &str, key: &str, value: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        key.to_string(),
        serde_json::Value::String(value.to_string()),
    );
    ToolCall::new("call-1", name, arguments)
}

#[test]
fn test_step_result() {
    let continue_result = StepResult::Continue;
    let completed_result = StepResult::Completed("Done".to_string());

    match continue_result {
        StepResult::Continue => {}
        _ => panic!("Expected Continue"),
    }

    match completed_result {
        StepResult::Completed(output) => assert_eq!(output, "Done"),
        _ => panic!("Expected Completed"),
    }
}

#[test]
fn test_subagent_settings_permission_blocks_denied_tool() {
    let temp_dir = TempDir::new().expect("temp dir");
    write_settings(
        &temp_dir,
        r#"{
            "permissions": {
                "deny": ["Write(secrets/**)"],
                "default_behavior": "allow"
            }
        }"#,
    );

    let result = SubAgentRunner::settings_permission_block(
        &tool_call("Write", "file_path", "secrets/key.txt"),
        temp_dir.path(),
    )
    .expect("denied tool should be blocked");

    assert!(!result.success);
    assert!(
        result
            .error
            .expect("error message")
            .contains("Permission denied by settings")
    );
}

#[test]
fn test_subagent_settings_permission_blocks_default_ask() {
    let temp_dir = TempDir::new().expect("temp dir");
    write_settings(
        &temp_dir,
        r#"{
            "permissions": {
                "allow": ["Read(src/**)"],
                "default_behavior": "ask"
            }
        }"#,
    );

    let result = SubAgentRunner::settings_permission_block(
        &tool_call("NotebookEdit", "notebook_path", "notebooks/private.ipynb"),
        temp_dir.path(),
    )
    .expect("ask decision should be blocked for subagents");

    assert!(!result.success);
    assert!(
        result
            .error
            .expect("error message")
            .contains("sub-agent tool calls cannot prompt")
    );
}

#[test]
fn test_subagent_settings_permission_allows_matching_rule() {
    let temp_dir = TempDir::new().expect("temp dir");
    write_settings(
        &temp_dir,
        r#"{
            "permissions": {
                "allow": ["Write(src/**)"],
                "default_behavior": "deny"
            }
        }"#,
    );

    let result = SubAgentRunner::settings_permission_block(
        &tool_call("Write", "file_path", "src/output.txt"),
        temp_dir.path(),
    );

    assert!(result.is_none());
}
