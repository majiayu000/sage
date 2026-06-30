use super::runner::{StepResult, SubAgentRunner, tool_context_for_child_tool_call};
use crate::llm::messages::LlmMessage;
use crate::tools::base::{Tool, ToolError};
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
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

struct NamedTool(&'static str);

#[async_trait]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "named test tool"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), vec![])
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success(&call.id, self.name(), "ok"))
    }
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
fn test_child_tool_context_carries_current_tool_scope() {
    let temp_dir = TempDir::new().expect("temp dir");
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(NamedTool("Task")),
        Arc::new(NamedTool("Read")),
        Arc::new(NamedTool("Task")),
    ];

    let context = tool_context_for_child_tool_call(temp_dir.path(), &tools, &[]);

    assert_eq!(context.working_directory, temp_dir.path());
    let parent_tools = context
        .metadata
        .get("parent_tools")
        .and_then(|value| value.as_array())
        .expect("parent_tools metadata");
    let names = parent_tools
        .iter()
        .map(|value| value.as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Read", "Task"]);
}

#[test]
fn test_child_tool_context_carries_nested_fork_context_messages()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool("Task"))];
    let messages = vec![
        LlmMessage::system("child role prompt"),
        LlmMessage::user("Parent subagent request"),
        LlmMessage::assistant("Parent subagent answer"),
        LlmMessage::tool("tool output", "tool-call", Some("Read")),
    ];

    let context = tool_context_for_child_tool_call(temp_dir.path(), &tools, &messages);

    let parent_context = context
        .metadata
        .get("parent_context")
        .and_then(|value| value.as_array())
        .ok_or("parent_context metadata")?;
    assert_eq!(parent_context.len(), 2);
    assert_eq!(parent_context[0]["role"], "user");
    assert_eq!(parent_context[0]["content"], "Parent subagent request");
    assert_eq!(parent_context[1]["role"], "assistant");
    assert_eq!(parent_context[1]["content"], "Parent subagent answer");
    Ok(())
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

#[test]
fn test_subagent_settings_permission_uses_inherited_tool_cwd() {
    let parent_dir = TempDir::new().expect("parent temp dir");
    let child_dir = TempDir::new().expect("child temp dir");
    write_settings(
        &parent_dir,
        r#"{
            "permissions": {
                "deny": ["Read(secrets/**)"],
                "default_behavior": "allow"
            }
        }"#,
    );

    let parent_result = SubAgentRunner::settings_permission_block(
        &tool_call("Read", "file_path", "secrets/key.txt"),
        parent_dir.path(),
    );
    let child_result = SubAgentRunner::settings_permission_block(
        &tool_call("Read", "file_path", "secrets/key.txt"),
        child_dir.path(),
    );

    assert!(
        parent_result.is_some(),
        "permission must be checked against the inherited cwd used by the tool"
    );
    assert!(
        child_result.is_none(),
        "checking against a distinct child cwd would miss the parent tool rule"
    );
}
