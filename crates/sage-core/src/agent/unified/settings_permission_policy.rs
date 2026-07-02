//! Shared policy helpers for settings-backed permission checks.

use crate::permissions::{
    PermissionAction, PermissionDecision, PermissionDecisionInput, PermissionProfileSource,
    PermissionRule, permission_pattern_matches,
};
use crate::settings::types::{Settings, SettingsPermissionBehavior};
use crate::tools::types::ToolCall;
use std::path::Path;

pub(super) fn deny_rules(settings: &Settings) -> Vec<PermissionRule> {
    let mut rules = settings
        .permissions
        .deny
        .iter()
        .cloned()
        .map(|pattern| PermissionRule::new(pattern, PermissionProfileSource::Local))
        .collect::<Vec<_>>();
    for managed in &settings.managed_configs {
        rules.extend(
            managed
                .config
                .permissions
                .deny
                .iter()
                .cloned()
                .map(|pattern| PermissionRule::new(pattern, PermissionProfileSource::Managed)),
        );
    }
    rules
}

pub(super) fn decision_reason(decision: &PermissionDecision) -> String {
    let Some(rule) = decision.matched_rule.as_ref() else {
        return decision.reason.clone();
    };
    format!(
        "{} source={:?} matched_rule={}",
        decision.reason, rule.source, rule.pattern
    )
}

pub(super) fn http_client_redirects_require_disabled(
    settings: &Settings,
    deny_rules: &[PermissionRule],
) -> bool {
    settings.permissions.default_behavior != SettingsPermissionBehavior::Allow
        || settings
            .managed_configs
            .iter()
            .any(|managed| managed.config.permissions.default_behavior.is_some())
        || has_http_client_url_permission_rule(
            settings
                .permissions
                .allow
                .iter()
                .map(String::as_str)
                .chain(deny_rules.iter().map(|rule| rule.pattern.as_str())),
        )
}

pub(super) fn http_client_may_follow_redirects(tool_call: &ToolCall) -> bool {
    tool_call.get_bool("follow_redirects").unwrap_or(true)
}

pub(super) fn is_confirmation_only_argument(key: &str) -> bool {
    key == "user_confirmed"
}

fn has_http_client_url_permission_rule<'a>(mut patterns: impl Iterator<Item = &'a str>) -> bool {
    patterns.any(|pattern| {
        pattern
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("http_client(")
    })
}

pub(super) fn settings_allow_outside_for_input(
    patterns: &[String],
    input: &PermissionDecisionInput,
) -> bool {
    if !matches!(input.action, PermissionAction::Filesystem) {
        return false;
    }
    let Some(path) = input.path.as_deref() else {
        return false;
    };
    let key = format!("{}({})", input.tool_name, path);
    patterns.iter().any(|pattern| {
        is_absolute_filesystem_permission(pattern) && permission_pattern_matches(pattern, &key)
    })
}

fn is_absolute_filesystem_permission(pattern: &str) -> bool {
    let Some((tool, argument)) = pattern
        .split_once('(')
        .and_then(|(tool, rest)| rest.rsplit_once(')').map(|(argument, _)| (tool, argument)))
    else {
        return false;
    };
    matches!(
        tool.trim().to_ascii_lowercase().as_str(),
        "read"
            | "write"
            | "edit"
            | "multiedit"
            | "multi_edit"
            | "grep"
            | "glob"
            | "notebookedit"
            | "notebook_edit"
    ) && Path::new(argument.trim()).is_absolute()
}
