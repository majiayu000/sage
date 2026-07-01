use super::*;
use crate::settings::types::{
    LoadedManagedConfig, ManagedConfig, ManagedConfigSource, ManagedConfigSourceKind,
    PermissionSettings, SettingsPermissionBehavior,
};
use std::path::{Path, PathBuf};

fn review_workspace_dir() -> &'static Path {
    Path::new("/workspace/sage")
}

fn review_tool_call(name: &str, arguments: serde_json::Value) -> ToolCall {
    let arguments = arguments
        .as_object()
        .map(|map| map.clone().into_iter().collect())
        .unwrap_or_default();
    ToolCall::new("call-1", name, arguments)
}

fn settings_with_managed_policy(policy: &str) -> Settings {
    let config = match ManagedConfig::parse_json(policy) {
        Ok(config) => config,
        Err(error) => panic!("expected managed policy: {error}"),
    };
    Settings {
        managed_configs: vec![LoadedManagedConfig {
            config,
            source: ManagedConfigSource {
                kind: ManagedConfigSourceKind::ProjectPolicy,
                path: PathBuf::from(".sage/managed.json"),
                sha256: "test".to_string(),
            },
        }],
        ..Default::default()
    }
}

#[test]
fn test_settings_permission_denies_multiedit_when_any_edit_path_matches() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["MultiEdit(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "multi_edit",
            serde_json::json!({
                "edits": [
                    {"file_path": "src/lib.rs", "old_string": "a", "new_string": "b"},
                    {"file_path": "secrets/key.txt", "old_string": "x", "new_string": "y"}
                ]
            }),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_requires_all_multiedit_paths_to_be_allowed() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["MultiEdit(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let allowed = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "multi_edit",
            serde_json::json!({
                "edits": [
                    {"file_path": "src/lib.rs", "old_string": "a", "new_string": "b"},
                    {"file_path": "src/main.rs", "old_string": "x", "new_string": "y"}
                ]
            }),
        ),
        review_workspace_dir(),
    );
    let denied = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "multi_edit",
            serde_json::json!({
                "edits": [
                    {"file_path": "src/lib.rs", "old_string": "a", "new_string": "b"},
                    {"file_path": "secrets/key.txt", "old_string": "x", "new_string": "y"}
                ]
            }),
        ),
        review_workspace_dir(),
    );

    assert_eq!(allowed, Some(SettingsPermissionDecision::Allow));
    assert!(matches!(denied, Some(SettingsPermissionDecision::Deny(_))));
}

#[test]
fn test_settings_permission_outside_allow_is_per_multiedit_path() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["MultiEdit(/tmp/allowed.txt)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "multi_edit",
            serde_json::json!({
                "edits": [
                    {"file_path": "/tmp/allowed.txt", "old_string": "a", "new_string": "b"},
                    {"file_path": "/etc/secret", "old_string": "x", "new_string": "y"}
                ]
            }),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_default_network_url_arguments() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec![
                "http_client(https://internal.example/**)".to_string(),
                "OpenBrowser(https://internal.example/**)".to_string(),
            ],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let http_client = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({"url": "https://internal.example/api"}),
        ),
        review_workspace_dir(),
    );
    let open_browser = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "OpenBrowser",
            serde_json::json!({"url": "https://internal.example/docs"}),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        http_client,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        open_browser,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_requires_http_client_redirects_disabled_for_url_rules() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["http_client(https://internal.example/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let redirecting_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({"url": "https://public.example"}),
        ),
        review_workspace_dir(),
    );
    let no_redirect_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({
                "url": "https://public.example",
                "follow_redirects": false
            }),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        redirecting_decision,
        Some(SettingsPermissionDecision::Deny(reason))
            if reason.contains("follow_redirects=false")
    ));
    assert_eq!(
        no_redirect_decision,
        Some(SettingsPermissionDecision::Allow)
    );
}

#[test]
fn test_settings_permission_requires_http_client_redirects_disabled_for_default_url_policy() {
    let settings = Settings {
        permissions: PermissionSettings {
            default_behavior: SettingsPermissionBehavior::Ask,
            default_behavior_set: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let redirecting_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({"url": "https://public.example"}),
        ),
        review_workspace_dir(),
    );
    let no_redirect_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({
                "url": "https://public.example",
                "follow_redirects": false
            }),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        redirecting_decision,
        Some(SettingsPermissionDecision::Deny(reason))
            if reason.contains("follow_redirects=false")
    ));
    assert!(matches!(
        no_redirect_decision,
        Some(SettingsPermissionDecision::Ask(_))
    ));
}

#[test]
fn test_managed_domain_policy_does_not_prompt_unrelated_tools() {
    let settings = settings_with_managed_policy(r#"{"permissions":{"network":{"enabled":false}}}"#);

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("bash", serde_json::json!({"command": "cargo test"})),
        review_workspace_dir(),
    );

    assert_eq!(decision, None);
}

#[test]
fn test_managed_search_denies_feed_overlap_preflights() {
    let mut settings = settings_with_managed_policy(
        r#"{"permissions":{"deny":["Grep(secrets/**)","Glob(secrets/**)"]}}"#,
    );
    settings.permissions.default_behavior = SettingsPermissionBehavior::Allow;

    let grep = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("grep", serde_json::json!({"pattern": "TOKEN"})),
        review_workspace_dir(),
    );
    let glob = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("glob", serde_json::json!({"pattern": "**/*"})),
        review_workspace_dir(),
    );

    assert!(matches!(grep, Some(SettingsPermissionDecision::Deny(_))));
    assert!(matches!(glob, Some(SettingsPermissionDecision::Deny(_))));
}

#[test]
fn test_managed_http_client_denies_require_redirects_disabled() {
    let mut settings = settings_with_managed_policy(
        r#"{"permissions":{"deny":["http_client(https://internal.example/**)"]}}"#,
    );
    settings.permissions.default_behavior = SettingsPermissionBehavior::Allow;

    let redirecting = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({"url": "https://public.example"}),
        ),
        review_workspace_dir(),
    );
    let no_redirect = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({
                "url": "https://public.example",
                "follow_redirects": false
            }),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        redirecting,
        Some(SettingsPermissionDecision::Deny(reason))
            if reason.contains("follow_redirects=false")
    ));
    assert_eq!(no_redirect, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_managed_network_ban_blocks_websearch() {
    let mut settings =
        settings_with_managed_policy(r#"{"permissions":{"network":{"enabled":false}}}"#);
    settings.permissions.default_behavior = SettingsPermissionBehavior::Allow;

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("web_search", serde_json::json!({"query": "public docs"})),
        review_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_managed_sandbox_required_does_not_fail_unknown_support() {
    let mut settings =
        settings_with_managed_policy(r#"{"permissions":{"sandbox":{"required":true}}}"#);
    settings.permissions.default_behavior = SettingsPermissionBehavior::Allow;

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("bash", serde_json::json!({"command": "cargo test"})),
        review_workspace_dir(),
    );

    assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_checks_http_client_save_to_file_path() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec![
                "http_client(https://trusted.example/**)".to_string(),
                "Write(downloads/**)".to_string(),
            ],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let allowed = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({
                "url": "https://trusted.example/archive",
                "follow_redirects": false,
                "save_to_file": "downloads/archive.json"
            }),
        ),
        review_workspace_dir(),
    );
    let denied = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({
                "url": "https://trusted.example/archive",
                "follow_redirects": false,
                "save_to_file": "secrets/token.txt"
            }),
        ),
        review_workspace_dir(),
    );

    assert_eq!(allowed, Some(SettingsPermissionDecision::Allow));
    assert!(matches!(denied, Some(SettingsPermissionDecision::Deny(_))));
}

#[test]
fn test_settings_permission_denies_save_path_before_prompting_for_url() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["Write(secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Ask,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "http_client",
            serde_json::json!({
                "url": "https://public.example/archive",
                "follow_redirects": false,
                "save_to_file": "secrets/token.json"
            }),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(reason)) if reason.contains("Write(secrets/**)")
    ));
}

#[test]
fn test_settings_permission_routes_log_analyzer_through_filesystem_guard() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["log_analyzer".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "log_analyzer",
            serde_json::json!({"file_path": ".sage/settings.local.json"}),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(reason)) if reason.contains("protected")
    ));
}

#[test]
fn test_settings_permission_routes_write_through_filesystem_guard() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Write(**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "write",
            serde_json::json!({"path": ".sage/settings.local.json"}),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(reason)) if reason.contains("protected")
    ));
}

#[test]
fn test_settings_permission_routes_search_tools_through_filesystem_guard() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Grep(**)".to_string(), "Glob(**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let grep = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("grep", serde_json::json!({"path": ".sage"})),
        review_workspace_dir(),
    );
    let grep_workspace = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("grep", serde_json::json!({"path": "."})),
        review_workspace_dir(),
    );
    let glob = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("glob", serde_json::json!({"pattern": ".sage/**"})),
        review_workspace_dir(),
    );
    let glob_search_path = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call(
            "glob",
            serde_json::json!({"path": ".", "pattern": ".sage/**"}),
        ),
        review_workspace_dir(),
    );

    assert!(matches!(
        grep,
        Some(SettingsPermissionDecision::Deny(reason)) if reason.contains("protected")
    ));
    assert!(matches!(
        grep_workspace,
        Some(SettingsPermissionDecision::Deny(reason)) if reason.contains("protected")
    ));
    assert!(matches!(
        glob,
        Some(SettingsPermissionDecision::Deny(reason)) if reason.contains("protected")
    ));
    assert!(matches!(
        glob_search_path,
        Some(SettingsPermissionDecision::Deny(reason)) if reason.contains("protected")
    ));
}

#[test]
fn test_settings_permission_allows_recursive_grep_directory_scope() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Grep(src/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("grep", serde_json::json!({"path": "src"})),
        review_workspace_dir(),
    );

    assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
}

#[test]
fn test_settings_permission_does_not_broaden_leading_glob_grep_allow_scope() {
    let settings = Settings {
        permissions: PermissionSettings {
            allow: vec!["Grep(**/secrets/**)".to_string()],
            default_behavior: SettingsPermissionBehavior::Deny,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("grep", serde_json::json!({"path": "src"})),
        review_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_denies_leading_glob_search_scope() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec![
                "Glob(**/secrets/**)".to_string(),
                "Grep(**/secrets/**)".to_string(),
            ],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let glob_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("glob", serde_json::json!({"pattern": "**/*"})),
        review_workspace_dir(),
    );
    let grep_decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &review_tool_call("grep", serde_json::json!({"path": "."})),
        review_workspace_dir(),
    );

    assert!(matches!(
        glob_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
    assert!(matches!(
        grep_decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}
