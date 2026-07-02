use super::*;
use crate::agent::ExecutionOptions;
use crate::agent::unified::CheckpointConfig;
use crate::checkpoints::CheckpointManager;
use crate::checkpoints::config::CheckpointManagerConfig;
use crate::config::Config;
use crate::input::{InputAutoResponse, InputChannel, InputRequestKind, InputResponse};
use crate::interrupt::InterruptManager;
use crate::tools::types::ToolCall;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

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
        Err(blocked) => {
            assert!(blocked.error.is_some_and(|error| {
                error.contains("Permission denied by settings") && error.contains("Bash(curl *)")
            }));
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

    let (result, executed_call) = executor
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
