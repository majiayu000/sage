use super::*;
use crate::settings::types::PermissionSettings;
use std::collections::HashMap;
use std::path::Path;

fn network_workspace_dir() -> &'static Path {
    Path::new("/workspace/sage")
}

fn network_web_fetch_call(url: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "url".to_string(),
        serde_json::Value::String(url.to_string()),
    );
    ToolCall::new("call-1", "web_fetch", arguments)
}

fn web_search_call(query: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "query".to_string(),
        serde_json::Value::String(query.to_string()),
    );
    ToolCall::new("call-1", "web_search", arguments)
}

#[test]
fn test_settings_permission_normalizes_webfetch_host_and_default_port() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["WebFetch(https://internal.example/private)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let decision = UnifiedExecutor::settings_permission_decision(
        &settings,
        &network_web_fetch_call("HTTPS://INTERNAL.EXAMPLE:443/private"),
        network_workspace_dir(),
    );

    assert!(matches!(
        decision,
        Some(SettingsPermissionDecision::Deny(_))
    ));
}

#[test]
fn test_settings_permission_matches_websearch_query_patterns() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec!["WebSearch(internal *)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    let blocked = UnifiedExecutor::settings_permission_decision(
        &settings,
        &web_search_call(" internal roadmap"),
        network_workspace_dir(),
    );
    let allowed = UnifiedExecutor::settings_permission_decision(
        &settings,
        &web_search_call("public docs"),
        network_workspace_dir(),
    );

    assert!(matches!(blocked, Some(SettingsPermissionDecision::Deny(_))));
    assert_eq!(allowed, Some(SettingsPermissionDecision::Allow));
}
