use super::*;
use crate::settings::types::PermissionSettings;
use std::collections::HashMap;
use std::path::Path;

fn network_workspace_dir() -> &'static Path {
    Path::new("/workspace/sage")
}

fn network_web_fetch_call(url: &str) -> ToolCall {
    network_url_call("web_fetch", url)
}

fn network_url_call(name: &str, url: &str) -> ToolCall {
    let mut arguments = HashMap::new();
    arguments.insert(
        "url".to_string(),
        serde_json::Value::String(url.to_string()),
    );
    ToolCall::new("call-1", name, arguments)
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
fn test_settings_permission_strips_url_fragments_before_matching() {
    let settings = Settings {
        permissions: PermissionSettings {
            deny: vec![
                "WebFetch(https://internal.example/private)".to_string(),
                "http_client(https://internal.example/private)".to_string(),
                "OpenBrowser(https://internal.example/private)".to_string(),
            ],
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    for tool_name in ["web_fetch", "http_client", "OpenBrowser"] {
        let mut call = network_url_call(tool_name, "https://internal.example/private#ignored");
        if tool_name == "http_client" {
            call.arguments.insert(
                "follow_redirects".to_string(),
                serde_json::Value::Bool(false),
            );
        }
        let decision = UnifiedExecutor::settings_permission_decision(
            &settings,
            &call,
            network_workspace_dir(),
        );

        assert!(
            matches!(decision, Some(SettingsPermissionDecision::Deny(_))),
            "{tool_name} should match deny rule without URL fragment"
        );
    }
}

#[test]
fn test_settings_permission_denies_blank_network_targets() {
    let settings = Settings {
        permissions: PermissionSettings {
            default_behavior: SettingsPermissionBehavior::Allow,
            ..Default::default()
        },
        ..Default::default()
    };

    for tool_name in ["web_fetch", "http_client", "OpenBrowser"] {
        let decision = UnifiedExecutor::settings_permission_decision(
            &settings,
            &network_url_call(tool_name, "   "),
            network_workspace_dir(),
        );

        assert!(
            matches!(
                decision,
                Some(SettingsPermissionDecision::Deny(reason))
                    if reason.contains("require a request target")
            ),
            "{tool_name} should require a non-empty network target"
        );
    }
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
