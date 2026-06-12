use super::*;
use crate::agent::ExecutionOptions;
use crate::config::Config;
use crate::input::{InputAutoResponse, InputChannel, InputRequestKind, InputResponse};
use crate::settings::types::PermissionSettings;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn workspace_dir() -> &'static Path {
    Path::new("/workspace/sage")
}

fn bash_call(command: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "command".to_string(),
        serde_json::Value::String(command.to_string()),
    );
    ToolCall::new("call-1", "bash", arguments)
}

fn read_call(path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "file_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", "read", arguments)
}

fn write_call(path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "file_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", "write", arguments)
}

fn path_call(tool_name: &str, path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", tool_name, arguments)
}

fn notebook_call(path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "notebook_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", "notebook_edit", arguments)
}

#[test]
fn test_settings_permission_denies_matching_rule() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Bash(echo *)".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("echo blocked"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_specific_command_against_actual_input() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Bash(rm -rf *)".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("rm -rf /tmp/foo"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_wildcard_deny_matches_any_tool() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["*".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("echo blocked"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_allows_matching_rule() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Bash(echo *)".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("echo allowed"),
        workspace_dir(),
    );

    assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_deny_precedes_allow() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Bash".to_string()],
            deny: vec!["Bash(echo *)".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("echo blocked"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_default_deny_blocks_unmatched_call() {
    let settings = Settings {
        permissions: PermissionSettings {
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("cargo test"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_empty_default_settings_do_not_force_permission_prompt() {
    let settings = Settings::default();

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("cargo test"),
        workspace_dir(),
    );

    assert_eq!(decision, None);
}

#[test]
fn test_settings_permission_matches_workspace_relative_absolute_path() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Read(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("/workspace/sage/src/lib.rs"),
        workspace_dir(),
    );

    assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_matches_grep_and_glob_paths() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec![
                "Grep(secrets/**)".to_string(),
                "Glob(secrets/**)".to_string(),
            ],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let grep_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "/workspace/sage/secrets/private.txt"),
        workspace_dir(),
    );
    let glob_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("glob", "/workspace/sage/secrets/private.txt"),
        workspace_dir(),
    );

    assert!(matches!(
        grep_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        glob_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_notebook_path() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["NotebookEdit(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &notebook_call("/workspace/sage/secrets/private.ipynb"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_legacy_permission_text_decision_parses_yes_and_no() {
    assert_eq!(
        UnifiedExecutor::legacy_permission_text_decision("yes"),
        Some(true)
    );
    assert_eq!(
        UnifiedExecutor::legacy_permission_text_decision(" y "),
        Some(true)
    );
    assert_eq!(
        UnifiedExecutor::legacy_permission_text_decision("no"),
        Some(false)
    );
    assert_eq!(
        UnifiedExecutor::legacy_permission_text_decision("continue"),
        None
    );
}

#[tokio::test]
async fn test_settings_permission_rechecks_modified_input_against_deny_rules() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir)?;
    fs::write(
        sage_dir.join("settings.local.json"),
        r#"{
            "permissions": {
                "deny": ["Write(secrets/**)"],
                "default_behavior": "ask"
            }
        }"#,
    )?;

    let modified_path = temp_dir
        .path()
        .join("secrets/key.txt")
        .to_string_lossy()
        .to_string();
    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            if matches!(&request.kind, InputRequestKind::Permission { .. }) {
                InputResponse::permission_granted_with_input(
                    request.id,
                    serde_json::json!({ "file_path": modified_path }),
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
    let result = executor
        .check_settings_permission(&write_call("src/tmp.txt"), &context)
        .await?;

    match result {
        Some(SettingsPermissionCheck::Blocked(result)) => {
            assert!(result.error.is_some_and(|error| {
                error.contains("Permission denied by settings")
                    && error.contains("Write(secrets/**)")
            }));
        }
        _ => panic!("modified denied path should be blocked by settings"),
    }

    Ok(())
}

#[test]
fn test_load_settings_strict_rejects_invalid_project_settings() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir)?;
    fs::write(sage_dir.join("settings.local.json"), "{ invalid json")?;

    let result = UnifiedExecutor::load_settings_strict(temp_dir.path());

    assert!(result.is_err());
    Ok(())
}
