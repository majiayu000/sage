//! Shared policy helpers for settings-backed permission checks.

use crate::permissions::{PermissionAction, PermissionDecisionInput, permission_pattern_matches};
use crate::settings::types::{Settings, SettingsPermissionBehavior};
use std::path::Path;

pub(super) fn deny_patterns(settings: &Settings) -> Vec<String> {
    let mut patterns = settings.permissions.deny.clone();
    for managed in &settings.managed_configs {
        patterns.extend(managed.config.permissions.deny.clone());
    }
    patterns
}

pub(super) fn http_client_redirects_require_disabled(
    settings: &Settings,
    managed_deny_patterns: &[String],
) -> bool {
    settings.permissions.default_behavior != SettingsPermissionBehavior::Allow
        || has_http_client_url_permission_rule(
            settings
                .permissions
                .allow
                .iter()
                .chain(settings.permissions.deny.iter())
                .chain(managed_deny_patterns.iter()),
        )
}

fn has_http_client_url_permission_rule<'a>(mut patterns: impl Iterator<Item = &'a String>) -> bool {
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
