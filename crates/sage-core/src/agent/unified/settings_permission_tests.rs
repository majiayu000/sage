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

fn grep_call_without_path(pattern: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "pattern".to_string(),
        serde_json::Value::String(pattern.to_string()),
    );
    ToolCall::new("call-1", "grep", arguments)
}

fn glob_call(pattern: &str, path: Option<&str>) -> ToolCall {
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

fn web_fetch_call(url: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "url".to_string(),
        serde_json::Value::String(url.to_string()),
    );
    ToolCall::new("call-1", "web_fetch", arguments)
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
fn test_settings_permission_trims_bash_command_before_matching_rules() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Bash(curl *)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call(" curl https://internal.example"),
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
fn test_explicit_ask_default_requires_prompt_without_rules() {
    let settings = Settings {
        permissions: PermissionSettings {
            default_behavior: SettingsPermissionBehavior::Ask,
            default_behavior_set: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &bash_call("echo needs prompt"),
        workspace_dir(),
    );

    assert!(matches!(decision, Some(SettingsPermissionDecision::Ask(_))));
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
fn test_settings_permission_keeps_outside_absolute_paths_distinct() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Read(tmp/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("/tmp/secret.txt"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_normalizes_windows_separators() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("src\\secret.txt"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
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
        &glob_call("secrets/**", None),
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
fn test_settings_permission_denies_broad_glob_overlapping_scoped_deny() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Glob(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let recursive_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("**/*", None),
        workspace_dir(),
    );
    let root_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("*", None),
        workspace_dir(),
    );

    assert!(matches!(
        recursive_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        root_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_denies_glob_search_path_overlapping_scoped_deny() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Glob(src/secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let broad_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("**/*", Some("/workspace/sage/src")),
        workspace_dir(),
    );
    let narrow_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("*.rs", Some("/workspace/sage/src")),
        workspace_dir(),
    );

    assert!(matches!(
        broad_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert_eq!(narrow_decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_denies_recursive_grep_overlapping_scoped_deny() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Grep(src/secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "src"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_denies_workspace_wide_grep_scope() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Grep(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &grep_call_without_path("token"),
        workspace_dir(),
    );
    let dot_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "."),
        workspace_dir(),
    );
    let root_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &path_call("grep", "/workspace/sage"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        dot_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        root_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_glob_path_joined_with_pattern() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Glob(src/secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &glob_call("secrets/**", Some("/workspace/sage/src")),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_webfetch_url() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["WebFetch(https://internal.example/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &web_fetch_call("https://internal.example/private"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_absolute_path_with_relative_working_dir() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Read(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };
    let cwd = std::env::current_dir().expect("current dir available");
    let absolute_path = cwd.join("src/lib.rs").to_string_lossy().to_string();

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call(&absolute_path),
        Path::new("."),
    );

    assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_normalizes_relative_path_components() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        settings_permission_paths::workspace_relative_path(
            "src/../secrets/key.txt",
            workspace_dir(),
        ),
        "secrets/key.txt"
    );

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("src/../secrets/key.txt"),
        workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[cfg(unix)]
#[test]
fn test_settings_permission_canonicalizes_existing_symlink_target() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    fs::create_dir(temp_dir.path().join("secrets"))?;
    fs::write(temp_dir.path().join("secrets/key.txt"), "secret")?;
    std::os::unix::fs::symlink(
        temp_dir.path().join("secrets"),
        temp_dir.path().join("public"),
    )?;

    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("public/key.txt"),
        temp_dir.path(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_settings_permission_resolves_symlink_before_parent_components() -> SageResult<()> {
    let temp_dir = TempDir::new()?;
    fs::create_dir(temp_dir.path().join("secrets"))?;
    fs::create_dir(temp_dir.path().join("secrets/subdir"))?;
    fs::write(temp_dir.path().join("secrets/key.txt"), "secret")?;
    std::os::unix::fs::symlink(
        temp_dir.path().join("secrets/subdir"),
        temp_dir.path().join("allowed"),
    )?;

    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Read(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &read_call("allowed/../key.txt"),
        temp_dir.path(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    Ok(())
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

#[tokio::test]
async fn test_settings_permission_strips_confirmation_fields_from_modified_input() -> SageResult<()>
{
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir)?;
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
                    serde_json::json!({
                        "command": "rm -rf target",
                        "user_confirmed": true
                    }),
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
        .check_settings_permission(&bash_call("echo ok"), &context)
        .await?;

    match result {
        Some(SettingsPermissionCheck::Allowed(tool_call)) => {
            assert_eq!(
                tool_call
                    .arguments
                    .get("command")
                    .and_then(|value| value.as_str()),
                Some("rm -rf target")
            );
            assert!(!tool_call.arguments.contains_key("user_confirmed"));
        }
        _ => panic!("modified allowed command should pass settings permission"),
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
