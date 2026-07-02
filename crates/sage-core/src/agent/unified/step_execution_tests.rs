use super::*;
use crate::agent::ExecutionOptions;
use crate::agent::unified::CheckpointConfig;
use crate::checkpoints::CheckpointManager;
use crate::checkpoints::config::CheckpointManagerConfig;
use crate::config::Config;
use crate::input::{InputAutoResponse, InputChannel, InputRequestKind, InputResponse};
use crate::interrupt::InterruptManager;
use crate::tools::types::ToolCall;
use crate::trajectory::{SessionEntry, SessionRecorder};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

fn bash_call(command: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "command".to_string(),
        serde_json::Value::String(command.to_string()),
    );
    ToolCall::new("call-1", "bash", arguments)
}

fn write_call(path: &str, content: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "file_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    arguments.insert(
        "content".to_string(),
        serde_json::Value::String(content.to_string()),
    );
    ToolCall::new("call-1", "write", arguments)
}

#[tokio::test]
async fn test_destructive_confirmation_edit_rechecks_settings() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir_all(&sage_dir)?;
    fs::write(
        sage_dir.join("settings.local.json"),
        r#"{
            "permissions": {
                "deny": ["Bash(curl *)"],
                "default_behavior": "allow"
            }
        }"#,
    )?;

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());

    let mut confirmed_call = bash_call("curl https://internal.example");
    UnifiedExecutor::mark_user_confirmed(&mut confirmed_call);

    let result = executor
        .recheck_settings_after_destructive_confirmation(confirmed_call, &context, true)
        .await;

    match result {
        Err((blocked, blocked_call)) => {
            assert!(blocked.error.is_some_and(|error| {
                error.contains("Permission denied by settings") && error.contains("Bash(curl *)")
            }));
            assert_eq!(
                blocked_call
                    .arguments
                    .get("command")
                    .and_then(|value| value.as_str()),
                Some("curl https://internal.example"),
                "blocked destructive recheck must preserve the edited call"
            );
        }
        Ok(_) => panic!("modified denied command should be blocked by settings"),
    }

    Ok(())
}

#[tokio::test]
async fn test_destructive_confirmation_settings_edit_requires_new_confirmation() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir_all(&sage_dir)?;
    fs::write(
        sage_dir.join("settings.local.json"),
        r#"{
            "permissions": {
                "default_behavior": "ask"
            }
        }"#,
    )?;

    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                InputResponse::permission_granted_with_input(
                    request.id,
                    serde_json::json!({ "command": "rm -rf target" }),
                )
            } else {
                InputResponse::cancelled(request.id)
            }
        })));

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_input_channel(input_channel);
    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());

    let mut confirmed_call = bash_call("rm -rf harmless");
    UnifiedExecutor::mark_user_confirmed(&mut confirmed_call);

    let result = executor
        .recheck_settings_after_destructive_confirmation(confirmed_call, &context, true)
        .await
        .expect("settings recheck should return an approved call");

    match result {
        SettingsRecheckAfterDestructiveConfirmation::NeedsDestructiveConfirmation(call) => {
            assert_eq!(
                call.arguments
                    .get("command")
                    .and_then(|value| value.as_str()),
                Some("rm -rf target")
            );
            assert!(!call.arguments.contains_key("user_confirmed"));
        }
        SettingsRecheckAfterDestructiveConfirmation::Ready(_) => {
            panic!("settings-edited destructive command must be confirmed again")
        }
    }

    Ok(())
}

/// Test double mirroring the bash tool's destructive-confirmation contract:
/// the first call reports a blocked destructive command, the confirmed retry
/// echoes the command it executed.
struct FakeDestructiveBash;

#[async_trait::async_trait]
impl crate::tools::base::Tool for FakeDestructiveBash {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "fake destructive bash for tests"
    }

    fn schema(&self) -> crate::tools::types::ToolSchema {
        crate::tools::types::ToolSchema::new("bash", "fake destructive bash for tests", vec![])
    }

    async fn execute(
        &self,
        call: &ToolCall,
    ) -> Result<crate::tools::types::ToolResult, crate::tools::base::ToolError> {
        let confirmed = call.get_bool("user_confirmed").unwrap_or(false);
        let command = call
            .arguments
            .get("command")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if !confirmed {
            return Ok(crate::tools::types::ToolResult::error(
                &call.id,
                "bash",
                "DESTRUCTIVE COMMAND BLOCKED: Confirmation required",
            ));
        }
        Ok(crate::tools::types::ToolResult::success(
            &call.id, "bash", command,
        ))
    }
}

struct RecordingAssertDestructiveBash {
    recorder_path: PathBuf,
    expected_command: String,
}

#[async_trait::async_trait]
impl crate::tools::base::Tool for RecordingAssertDestructiveBash {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "fake destructive bash with recording assertion"
    }

    fn schema(&self) -> crate::tools::types::ToolSchema {
        crate::tools::types::ToolSchema::new(
            "bash",
            "fake destructive bash with recording assertion",
            vec![],
        )
    }

    async fn execute(
        &self,
        call: &ToolCall,
    ) -> Result<crate::tools::types::ToolResult, crate::tools::base::ToolError> {
        let confirmed = call.get_bool("user_confirmed").unwrap_or(false);
        let command = call
            .arguments
            .get("command")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if !confirmed {
            return Ok(crate::tools::types::ToolResult::error(
                &call.id,
                "bash",
                "DESTRUCTIVE COMMAND BLOCKED: Confirmation required",
            ));
        }

        let entries = SessionRecorder::load_entries(&self.recorder_path)
            .await
            .expect("session entries should load while tool executes");
        let saw_expected_call = entries.iter().any(|entry| {
            matches!(
                entry,
                SessionEntry::ToolCall {
                    tool_name,
                    tool_input,
                    ..
                } if tool_name == "bash"
                    && tool_input.get("command").and_then(|value| value.as_str())
                        == Some(self.expected_command.as_str())
                    && !tool_input
                        .as_object()
                        .is_some_and(|input| input.contains_key("user_confirmed"))
            )
        });
        assert!(
            saw_expected_call,
            "edited command must be recorded before final execution"
        );

        Ok(crate::tools::types::ToolResult::success(
            &call.id, "bash", command,
        ))
    }
}

#[tokio::test]
async fn test_destructive_confirmation_edit_propagates_executed_call() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir_all(&sage_dir)?;
    fs::write(
        sage_dir.join("settings.local.json"),
        r#"{
            "permissions": {
                "default_behavior": "allow"
            }
        }"#,
    )?;

    // Auto-approve the destructive prompt while editing the command.
    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                InputResponse::permission_granted_with_input(
                    request.id,
                    serde_json::json!({ "command": "rm -rf edited-target" }),
                )
            } else {
                InputResponse::cancelled(request.id)
            }
        })));

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_input_channel(input_channel);
    executor
        .tool_orchestrator
        .tool_executor
        .register_tool(Arc::new(FakeDestructiveBash));
    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let interrupt_manager = InterruptManager::new();
    let task_scope = interrupt_manager.create_task_scope();

    let (result, executed_call, recorded_call) = executor
        .execute_with_permission_check(
            &bash_call("rm -rf original-target"),
            &context,
            task_scope.token().clone(),
        )
        .await;

    assert!(
        result.success,
        "edited command should execute: {:?}",
        result.error
    );
    assert_eq!(
        executed_call
            .arguments
            .get("command")
            .and_then(|value| value.as_str()),
        Some("rm -rf edited-target"),
        "callers must observe the user-edited command, not the original request"
    );
    assert!(
        !executed_call.arguments.contains_key("user_confirmed"),
        "internal confirmation marker must not leak to observers"
    );
    assert!(
        recorded_call,
        "confirmed destructive execution records before running"
    );

    Ok(())
}

#[tokio::test]
async fn test_destructive_confirmation_records_edited_call_before_execution() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir_all(&sage_dir)?;
    fs::write(
        sage_dir.join("settings.local.json"),
        r#"{
            "permissions": {
                "default_behavior": "allow"
            }
        }"#,
    )?;

    let edited_target = temp_dir.path().join("edited-target");
    let edited_command = format!("rm -rf {}", edited_target.display());
    let input_command = edited_command.clone();
    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                InputResponse::permission_granted_with_input(
                    request.id,
                    serde_json::json!({ "command": input_command }),
                )
            } else {
                InputResponse::cancelled(request.id)
            }
        })));

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_input_channel(input_channel);
    let recorder = Arc::new(Mutex::new(SessionRecorder::new(temp_dir.path())?));
    let recorder_path = recorder.lock().await.file_path().to_path_buf();
    executor.set_session_recorder(Arc::clone(&recorder));
    executor
        .tool_orchestrator
        .tool_executor
        .register_tool(Arc::new(RecordingAssertDestructiveBash {
            recorder_path: recorder_path.clone(),
            expected_command: edited_command.clone(),
        }));

    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let interrupt_manager = InterruptManager::new();
    let task_scope = interrupt_manager.create_task_scope();

    let result = executor
        .execute_single_tool(&bash_call("rm -rf original-target"), &context, &task_scope)
        .await?;

    assert!(result.success, "edited command should execute");
    let entries = SessionRecorder::load_entries(recorder_path).await?;
    let matching_tool_calls = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry,
                SessionEntry::ToolCall {
                    tool_name,
                    tool_input,
                    ..
                } if tool_name == "bash"
                    && tool_input.get("command").and_then(|value| value.as_str())
                        == Some(edited_command.as_str())
            )
        })
        .count();
    assert_eq!(
        matching_tool_calls, 1,
        "edited command should be recorded exactly once"
    );

    Ok(())
}

#[tokio::test]
async fn test_destructive_confirmation_tracks_edited_bash_rm_target() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let edited_target = temp_dir.path().join("edited-target.txt");
    fs::write(&edited_target, "before")?;

    let edited_command = format!("rm -rf {}", edited_target.display());
    let input_command = edited_command.clone();
    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                InputResponse::permission_granted_with_input(
                    request.id,
                    serde_json::json!({ "command": input_command }),
                )
            } else {
                InputResponse::cancelled(request.id)
            }
        })));

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_input_channel(input_channel);
    executor
        .tool_orchestrator
        .tool_executor
        .register_tool(Arc::new(FakeDestructiveBash));
    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let interrupt_manager = InterruptManager::new();
    let task_scope = interrupt_manager.create_task_scope();

    let result = executor
        .execute_single_tool(&bash_call("rm -rf original-target"), &context, &task_scope)
        .await?;

    assert!(result.success, "edited command should execute");
    assert!(
        executor
            .session_manager()
            .file_tracker()
            .tracked_paths()
            .contains(&edited_target),
        "edited Bash rm target should be tracked for undo"
    );

    Ok(())
}

#[test]
fn test_bash_rm_targets_stop_at_shell_separator() {
    let targets = bash_rm_targets(&bash_call("rm edited-target; echo done"));
    assert_eq!(targets, vec!["edited-target"]);
}

#[tokio::test]
async fn test_destructive_confirmation_tracks_expanded_rm_glob_targets() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let log_a = temp_dir.path().join("a.log");
    let log_b = temp_dir.path().join("b.log");
    let text_file = temp_dir.path().join("keep.txt");
    fs::write(&log_a, "a")?;
    fs::write(&log_b, "b")?;
    fs::write(&text_file, "keep")?;

    let edited_command = format!("rm {}/*.log", temp_dir.path().display());
    let input_command = edited_command.clone();
    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                InputResponse::permission_granted_with_input(
                    request.id,
                    serde_json::json!({ "command": input_command }),
                )
            } else {
                InputResponse::cancelled(request.id)
            }
        })));

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_input_channel(input_channel);
    executor
        .tool_orchestrator
        .tool_executor
        .register_tool(Arc::new(FakeDestructiveBash));
    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let interrupt_manager = InterruptManager::new();
    let task_scope = interrupt_manager.create_task_scope();

    let result = executor
        .execute_single_tool(&bash_call("rm original-target"), &context, &task_scope)
        .await?;

    let tracked = executor.session_manager().file_tracker().tracked_paths();
    assert!(result.success, "edited command should execute");
    assert!(
        tracked.contains(&log_a),
        "first glob match should be tracked"
    );
    assert!(
        tracked.contains(&log_b),
        "second glob match should be tracked"
    );
    assert!(
        !tracked.contains(&text_file),
        "non-matching file should not be tracked"
    );

    Ok(())
}

#[tokio::test]
async fn test_destructive_confirmation_tracks_rm_directory_children() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let edited_dir = temp_dir.path().join("edited-dir");
    let nested_dir = edited_dir.join("nested");
    fs::create_dir_all(&nested_dir)?;
    let child = edited_dir.join("child.txt");
    let nested_child = nested_dir.join("nested.txt");
    fs::write(&child, "child")?;
    fs::write(&nested_child, "nested")?;

    let edited_command = format!("rm -rf {}", edited_dir.display());
    let input_command = edited_command.clone();
    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                InputResponse::permission_granted_with_input(
                    request.id,
                    serde_json::json!({ "command": input_command }),
                )
            } else {
                InputResponse::cancelled(request.id)
            }
        })));

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_input_channel(input_channel);
    executor
        .tool_orchestrator
        .tool_executor
        .register_tool(Arc::new(FakeDestructiveBash));
    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let interrupt_manager = InterruptManager::new();
    let task_scope = interrupt_manager.create_task_scope();

    let result = executor
        .execute_single_tool(&bash_call("rm -rf original-target"), &context, &task_scope)
        .await?;

    let tracked = executor.session_manager().file_tracker().tracked_paths();
    assert!(result.success, "edited command should execute");
    assert!(
        tracked.contains(&child),
        "directory child should be tracked"
    );
    assert!(
        tracked.contains(&nested_child),
        "nested directory child should be tracked"
    );

    Ok(())
}

#[tokio::test]
async fn test_settings_denial_skips_post_execution_rollback() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let tracked_file = temp_dir.path().join("tracked.txt");
    fs::write(&tracked_file, "original")?;

    let manager = Arc::new(CheckpointManager::new(CheckpointManagerConfig::new(
        temp_dir.path(),
    )));
    let checkpoint = manager
        .create_pre_tool_checkpoint("Write", std::slice::from_ref(&tracked_file))
        .await?;
    fs::write(&tracked_file, "modified")?;

    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir_all(&sage_dir)?;
    fs::write(
        sage_dir.join("settings.local.json"),
        r#"{
            "permissions": {
                "deny": ["Write(secrets/**)"],
                "default_behavior": "allow"
            }
        }"#,
    )?;

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor
        .tool_orchestrator
        .set_checkpoint_manager(Arc::clone(&manager));
    executor
        .tool_orchestrator
        .set_checkpoint_config(CheckpointConfig::with_auto_rollback());
    {
        let mut last_checkpoint_id = executor.tool_orchestrator.last_checkpoint_id.write().await;
        *last_checkpoint_id = Some(checkpoint.id);
    }

    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let interrupt_manager = InterruptManager::new();
    let task_scope = interrupt_manager.create_task_scope();
    let result = executor
        .execute_single_tool(
            &write_call("secrets/blocked.txt", "secret"),
            &context,
            &task_scope,
        )
        .await?;

    assert!(!result.success);
    assert!(result.error.is_some_and(|error| {
        error.contains("Permission denied by settings") && error.contains("Write(secrets/**)")
    }));
    assert_eq!(fs::read_to_string(&tracked_file)?, "modified");
    assert!(
        executor
            .tool_orchestrator
            .last_checkpoint_id()
            .await
            .is_some()
    );

    Ok(())
}
