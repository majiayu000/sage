use super::*;
use crate::agent::ExecutionOptions;
use crate::config::Config;
use crate::tools::types::ToolCall;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn bash_call(command: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "command".to_string(),
        serde_json::Value::String(command.to_string()),
    );
    ToolCall::new("call-1", "bash", arguments)
}

#[tokio::test]
async fn test_destructive_confirmation_edit_rechecks_settings() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir)?;
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
