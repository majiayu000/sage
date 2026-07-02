use super::settings_permission_test_support::workspace_dir;
use super::*;
use crate::agent::ExecutionOptions;
use crate::config::Config;
use crate::diagnostics::{AuditDecisionKind, audit_summaries_from_events, global_diagnostics};
use crate::input::{InputAutoResponse, InputChannel, InputRequestKind, InputResponse};
use crate::permissions::PermissionProfileSource;
use crate::settings::types::PermissionSettings;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tempfile::TempDir;

fn bash_call(command: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "command".to_string(),
        serde_json::Value::String(command.to_string()),
    );
    ToolCall::new("call-1", "bash", arguments)
}

fn write_call(path: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "file_path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    ToolCall::new("call-1", "write", arguments)
}

fn web_fetch_call(url: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "url".to_string(),
        serde_json::Value::String(url.to_string()),
    );
    ToolCall::new("call-1", "web_fetch", arguments)
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
#[serial]
async fn test_settings_permission_rechecks_modified_input_against_deny_rules() -> SageResult<()> {
    global_diagnostics().clear();
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
        Some(SettingsPermissionCheck::Blocked { result, tool_call }) => {
            assert!(result.error.is_some_and(|error| {
                error.contains("Permission denied by settings")
                    && error.contains("Write(secrets/**)")
            }));
            assert!(
                tool_call
                    .arguments
                    .get("file_path")
                    .and_then(|value| value.as_str())
                    .is_some_and(|path| path.ends_with("secrets/key.txt")),
                "blocked settings result should preserve the edited call"
            );
        }
        _ => panic!("modified denied path should be blocked by settings"),
    }

    let summaries = audit_summaries_from_events(&global_diagnostics().snapshot());
    let summary = summaries
        .iter()
        .find(|summary| summary.reason.contains("Write(secrets/**)"))
        .expect("settings denial should be captured in diagnostics audit summary");
    assert_eq!(summary.decision, AuditDecisionKind::Deny);
    assert_eq!(summary.source, Some(PermissionProfileSource::Local));
    global_diagnostics().clear();

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_managed_default_denial_records_audit_summary() -> SageResult<()> {
    global_diagnostics().clear();
    let temp_dir = TempDir::new()?;
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir)?;
    fs::write(
        sage_dir.join("managed.json"),
        r#"{
            "permissions": {
                "default_behavior": "deny"
            }
        }"#,
    )?;

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive().with_working_directory(temp_dir.path());
    let mut executor = UnifiedExecutor::with_options(config, options)?;

    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let result = executor
        .check_settings_permission(&bash_call("curl https://example.com"), &context)
        .await?;

    assert!(matches!(
        result,
        Some(SettingsPermissionCheck::Blocked { .. })
    ));
    let summaries = audit_summaries_from_events(&global_diagnostics().snapshot());
    assert!(summaries.iter().any(|summary| {
        summary.reason.contains("source=Some(Managed)")
            && summary.decision == AuditDecisionKind::Deny
            && summary.source == Some(PermissionProfileSource::Managed)
    }));
    global_diagnostics().clear();

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

#[tokio::test]
async fn test_settings_permission_prompt_uses_execution_timeout() -> SageResult<()> {
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

    let observed_timeout_ms = Arc::new(AtomicU64::new(0));
    let observed_timeout_for_response = Arc::clone(&observed_timeout_ms);
    let input_channel =
        InputChannel::non_interactive(InputAutoResponse::Custom(Arc::new(move |request| {
            let timeout_ms = request
                .timeout
                .map(|timeout| timeout.as_millis() as u64)
                .unwrap_or(0);
            observed_timeout_for_response.store(timeout_ms, Ordering::SeqCst);
            InputResponse::permission_denied(request.id, Some("no".to_string()))
        })));

    let mut config = Config::default();
    config.default_provider = "ollama".to_string();
    let options = ExecutionOptions::interactive()
        .with_working_directory(temp_dir.path())
        .with_prompt_timeout(Duration::from_secs(7));
    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_input_channel(input_channel);

    let context = ToolExecutionContext::new("session", temp_dir.path().to_path_buf());
    let result = executor
        .check_settings_permission(&bash_call("echo ok"), &context)
        .await?;

    assert!(matches!(
        result,
        Some(SettingsPermissionCheck::Blocked { .. })
    ));
    assert_eq!(observed_timeout_ms.load(Ordering::SeqCst), 7_000);

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
