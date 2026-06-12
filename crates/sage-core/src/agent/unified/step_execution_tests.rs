use super::*;
use crate::agent::ExecutionOptions;
use crate::agent::unified::CheckpointConfig;
use crate::checkpoints::CheckpointManager;
use crate::checkpoints::config::CheckpointManagerConfig;
use crate::config::Config;
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
