use super::decision_engine_keys::{
    bash_aware_allow_matches, bash_aware_deny_matches, normalize_path, path_is_at_or_under,
    rule_match_keys,
};
use super::{PermissionBehavior, PermissionProfile, PermissionProfileSource, PermissionRule};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    Tool,
    Filesystem,
    Network,
    Exec,
    Sandbox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxSupport {
    Unknown,
    Supported,
    Unsupported,
}

impl Default for SandboxSupport {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionPreflight {
    pub reason: String,
    pub matched_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_rule_source: Option<PermissionProfileSource>,
}

#[rustfmt::skip]
impl PermissionPreflight {
    pub fn new(reason: impl Into<String>, matched_rule: Option<String>) -> Self { Self { reason: reason.into(), matched_rule, matched_rule_source: None } }
    pub fn with_source(mut self, source: PermissionProfileSource) -> Self { self.matched_rule_source = Some(source); self }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDecisionInput {
    pub action: PermissionAction,
    pub tool_name: String,
    pub permission_keys: Vec<String>,
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    pub network_target: Option<String>,
    pub requires_sandbox: bool,
    #[serde(default)]
    pub sandbox_support: SandboxSupport,
    pub preflight_denies: Vec<PermissionPreflight>,
    pub scoped_allows: Vec<PermissionPreflight>,
}

impl PermissionDecisionInput {
    pub fn new(
        action: PermissionAction,
        tool_name: impl Into<String>,
        permission_keys: Vec<String>,
    ) -> Self {
        Self {
            action,
            tool_name: tool_name.into(),
            permission_keys,
            path: None,
            working_directory: None,
            network_target: None,
            requires_sandbox: false,
            sandbox_support: SandboxSupport::Unknown,
            preflight_denies: Vec::new(),
            scoped_allows: Vec::new(),
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_working_directory(mut self, working_directory: impl Into<String>) -> Self {
        self.working_directory = Some(working_directory.into());
        self
    }

    pub fn with_network_target(mut self, target: impl Into<String>) -> Self {
        let target = target.into();
        if target.trim().is_empty() {
            self.network_target = None;
        } else {
            self.network_target = Some(target);
        }
        self
    }

    pub fn with_required_sandbox(mut self, support: SandboxSupport) -> Self {
        self.requires_sandbox = true;
        self.sandbox_support = support;
        self
    }

    pub fn with_preflight_denies(mut self, denies: Vec<PermissionPreflight>) -> Self {
        self.preflight_denies = denies;
        self
    }

    pub fn with_scoped_allows(mut self, allows: Vec<PermissionPreflight>) -> Self {
        self.scoped_allows = allows;
        self
    }

    fn audit_key(&self, rule_match_keys: &[String]) -> String {
        self.permission_keys
            .first()
            .cloned()
            .or_else(|| rule_match_keys.first().cloned())
            .or_else(|| self.structured_permission_key())
            .unwrap_or_else(|| self.tool_name.clone())
    }

    fn structured_permission_key(&self) -> Option<String> {
        match self.action {
            PermissionAction::Filesystem => self
                .path
                .as_ref()
                .map(|path| format!("{}({})", self.tool_name, path)),
            PermissionAction::Network => self
                .network_target
                .as_ref()
                .map(|target| format!("{}({})", self.tool_name, target)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecisionKind {
    Allow,
    Deny,
    Ask,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub kind: PermissionDecisionKind,
    pub reason: String,
    pub audit_key: String,
    pub matched_rule: Option<PermissionRule>,
}

impl PermissionDecision {
    fn new(
        kind: PermissionDecisionKind,
        audit_key: impl Into<String>,
        reason: impl Into<String>,
        matched_rule: Option<PermissionRule>,
    ) -> Self {
        Self {
            kind,
            reason: reason.into(),
            audit_key: audit_key.into(),
            matched_rule,
        }
    }
}

pub struct PermissionDecisionEngine {
    profile: PermissionProfile,
}

impl PermissionDecisionEngine {
    pub fn new(profile: PermissionProfile) -> Self {
        Self { profile }
    }

    pub fn decide(&self, input: PermissionDecisionInput) -> PermissionDecision {
        let rule_match_keys = rule_match_keys(&self.profile, &input);
        let audit_key = input.audit_key(&rule_match_keys);
        let sources = &self.profile.domain_sources;

        if matches!(input.action, PermissionAction::Network) && !self.profile.network.enabled {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                format!("network access is disabled source={:?}", sources.network),
                None,
            );
        }

        if matches!(input.action, PermissionAction::Network)
            && input
                .network_target
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                "network permission decisions require a request target",
                None,
            );
        }

        if matches!(input.action, PermissionAction::Exec) && !self.profile.exec.enabled {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                format!("exec disabled source={:?}", sources.exec),
                None,
            );
        }

        if matches!(input.action, PermissionAction::Filesystem) && input.path.is_none() {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                "filesystem permission decisions require a request path",
                None,
            );
        }

        if let Some(path) = input.path.as_deref() {
            let working_directory = input.working_directory.as_deref();
            if self.path_is_protected(path, working_directory)
                || self.search_scope_touches_protected(&input, path, working_directory)
            {
                return PermissionDecision::new(
                    PermissionDecisionKind::Deny,
                    audit_key,
                    format!("protected path source={:?}: '{}'", sources.filesystem, path),
                    None,
                );
            }

            if !self.profile.filesystem.allow_outside_workspace {
                if self.profile.filesystem.workspace_roots.is_empty() {
                    return PermissionDecision::new(
                        PermissionDecisionKind::Deny,
                        audit_key,
                        format!(
                            "no workspace roots are configured source={:?}: '{}'",
                            sources.filesystem, path
                        ),
                        None,
                    );
                }

                if !self.path_is_in_workspace(path, working_directory) {
                    return PermissionDecision::new(
                        PermissionDecisionKind::Deny,
                        audit_key,
                        format!("path '{}' is outside configured workspace roots", path),
                        None,
                    );
                }
            }
        }

        if (input.requires_sandbox || self.profile.sandbox.required)
            && input.sandbox_support != SandboxSupport::Supported
        {
            return PermissionDecision::new(
                PermissionDecisionKind::Unsupported,
                audit_key,
                format!("sandbox unsupported source={:?}", sources.sandbox),
                None,
            );
        }

        if let Some(deny) = input.preflight_denies.first() {
            let source = deny.matched_rule_source.unwrap_or(self.profile.source);
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                deny.reason.clone(),
                deny.matched_rule
                    .as_ref()
                    .map(|pattern| PermissionRule::new(pattern.clone(), source)),
            );
        }

        if let Some(rule) = self.matching_rule(&self.profile.deny, &rule_match_keys) {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                format!("matched deny rule '{}'", rule.pattern),
                Some(rule.clone()),
            );
        }

        let supplied_allow_matches: Vec<&PermissionRule> = input
            .permission_keys
            .iter()
            .filter_map(|key| self.matching_rule_for_key(&self.profile.allow, key))
            .collect();
        let structured_allow_matches: Vec<&PermissionRule> = rule_match_keys
            .iter()
            .filter(|key| !input.permission_keys.contains(key))
            .filter_map(|key| self.matching_rule_for_key(&self.profile.allow, key))
            .collect();
        let allow_matched = if input.permission_keys.is_empty() {
            !structured_allow_matches.is_empty()
        } else {
            supplied_allow_matches.len() == input.permission_keys.len()
                || !structured_allow_matches.is_empty()
        };
        if !rule_match_keys.is_empty() && allow_matched {
            return PermissionDecision::new(
                PermissionDecisionKind::Allow,
                audit_key,
                "matched allow rule",
                supplied_allow_matches
                    .first()
                    .or_else(|| structured_allow_matches.first())
                    .map(|rule| (*rule).clone()),
            );
        }

        if let Some(allow) = input.scoped_allows.first() {
            return PermissionDecision::new(
                PermissionDecisionKind::Allow,
                audit_key,
                allow.reason.clone(),
                allow
                    .matched_rule
                    .as_ref()
                    .map(|pattern| PermissionRule::new(pattern.clone(), self.profile.source)),
            );
        }

        match self.profile.default_behavior {
            PermissionBehavior::Allow => PermissionDecision::new(
                PermissionDecisionKind::Allow,
                audit_key,
                "default permission behavior is allow",
                None,
            ),
            PermissionBehavior::Deny => PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key.clone(),
                format!(
                    "no allow rule matched '{}' source={:?}",
                    audit_key, self.profile.default_behavior_source
                ),
                None,
            ),
            PermissionBehavior::Ask => PermissionDecision::new(
                PermissionDecisionKind::Ask,
                audit_key.clone(),
                format!(
                    "No permission rule matched '{}'. source={:?}",
                    audit_key, self.profile.default_behavior_source
                ),
                None,
            ),
        }
    }

    fn matching_rule<'a>(
        &'a self,
        rules: &'a [PermissionRule],
        keys: &[String],
    ) -> Option<&'a PermissionRule> {
        rules.iter().find(|rule| {
            keys.iter()
                .any(|key| bash_aware_deny_matches(&rule.pattern, key))
        })
    }

    fn matching_rule_for_key<'a>(
        &'a self,
        rules: &'a [PermissionRule],
        key: &str,
    ) -> Option<&'a PermissionRule> {
        rules
            .iter()
            .find(|rule| bash_aware_allow_matches(&rule.pattern, key))
    }

    fn path_is_in_workspace(&self, path: &str, working_directory: Option<&str>) -> bool {
        let path = normalize_path(path, working_directory);
        self.profile
            .filesystem
            .workspace_roots
            .iter()
            .map(|root| normalize_path(root, working_directory))
            .any(|root| path_is_at_or_under(&path, &root))
    }

    fn path_is_protected(&self, path: &str, working_directory: Option<&str>) -> bool {
        let path = normalize_path(path, working_directory);
        self.protected_path_roots(working_directory)
            .iter()
            .any(|protected| path_is_at_or_under(&path, protected))
    }

    fn search_scope_touches_protected(
        &self,
        input: &PermissionDecisionInput,
        path: &str,
        working_directory: Option<&str>,
    ) -> bool {
        if !matches!(input.action, PermissionAction::Filesystem)
            || !matches!(
                input.tool_name.to_ascii_lowercase().as_str(),
                "grep" | "glob"
            )
        {
            return false;
        }

        let scope = if input.tool_name.eq_ignore_ascii_case("glob") {
            glob_static_scope(path)
        } else {
            PathBuf::from(path)
        };
        let scope = normalize_path(scope, working_directory);
        self.protected_path_roots(working_directory)
            .iter()
            .any(|protected| {
                path_is_at_or_under(&scope, protected) || path_is_at_or_under(protected, &scope)
            })
    }

    fn protected_path_roots(&self, working_directory: Option<&str>) -> Vec<PathBuf> {
        self.profile
            .filesystem
            .protected_paths
            .iter()
            .flat_map(|protected| {
                if Path::new(protected).is_absolute() {
                    vec![normalize_path(protected, None)]
                } else {
                    self.profile
                        .filesystem
                        .workspace_roots
                        .iter()
                        .map(|root| {
                            normalize_path(
                                normalize_path(root, working_directory).join(protected),
                                None,
                            )
                        })
                        .collect()
                }
            })
            .collect()
    }
}

fn glob_static_scope(pattern: &str) -> PathBuf {
    let mut scope = PathBuf::new();
    for component in Path::new(pattern).components() {
        let component_text = component.as_os_str().to_string_lossy();
        if component_text
            .chars()
            .any(|ch| matches!(ch, '*' | '?' | '[' | ']' | '{' | '}'))
        {
            break;
        }
        if component_text == "." {
            continue;
        }
        scope.push(component.as_os_str());
    }

    if scope.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        scope
    }
}

#[cfg(test)]
#[path = "decision_engine_allow_tests.rs"]
mod allow_tests;

#[cfg(test)]
#[path = "decision_engine_network_tests.rs"]
mod network_tests;

#[cfg(test)]
#[path = "decision_engine_path_tests.rs"]
mod path_tests;

#[cfg(test)]
#[path = "decision_engine_tests.rs"]
mod tests;
