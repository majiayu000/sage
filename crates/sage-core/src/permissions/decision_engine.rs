use super::{PermissionBehavior, PermissionProfile, PermissionRule};
use crate::tools::permission::PermissionCache;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

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
    Supported,
    Unsupported,
}

impl Default for SandboxSupport {
    fn default() -> Self {
        Self::Supported
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionPreflight {
    pub reason: String,
    pub matched_rule: Option<String>,
}

impl PermissionPreflight {
    pub fn new(reason: impl Into<String>, matched_rule: Option<String>) -> Self {
        Self {
            reason: reason.into(),
            matched_rule,
        }
    }
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
            sandbox_support: SandboxSupport::Supported,
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
        self.network_target = Some(target.into());
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

    fn audit_key(&self) -> String {
        self.permission_keys
            .first()
            .cloned()
            .or_else(|| self.structured_permission_key())
            .unwrap_or_else(|| self.tool_name.clone())
    }

    fn rule_match_keys(&self) -> Vec<String> {
        if self.permission_keys.is_empty() {
            self.structured_permission_key()
                .map(|key| vec![key])
                .unwrap_or_default()
        } else {
            self.permission_keys.clone()
        }
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
        let audit_key = input.audit_key();
        let rule_match_keys = input.rule_match_keys();

        if matches!(input.action, PermissionAction::Network) && !self.profile.network.enabled {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                "network access is disabled by permission profile",
                None,
            );
        }

        if matches!(input.action, PermissionAction::Exec) && !self.profile.exec.enabled {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                "process execution is disabled by permission profile",
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
            if self.path_is_protected(path, working_directory) {
                return PermissionDecision::new(
                    PermissionDecisionKind::Deny,
                    audit_key,
                    format!("path '{}' is protected by permission profile", path),
                    None,
                );
            }

            if !self.profile.filesystem.allow_outside_workspace {
                if self.profile.filesystem.workspace_roots.is_empty() {
                    return PermissionDecision::new(
                        PermissionDecisionKind::Deny,
                        audit_key,
                        format!(
                            "path '{}' cannot be checked because no workspace roots are configured",
                            path
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
            && input.sandbox_support == SandboxSupport::Unsupported
        {
            return PermissionDecision::new(
                PermissionDecisionKind::Unsupported,
                audit_key,
                "requested sandbox is not supported on this platform",
                None,
            );
        }

        if let Some(deny) = input.preflight_denies.first() {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                deny.reason.clone(),
                deny.matched_rule
                    .as_ref()
                    .map(|pattern| PermissionRule::new(pattern.clone(), self.profile.source)),
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

        let allow_matches: Vec<&PermissionRule> = rule_match_keys
            .iter()
            .filter_map(|key| self.matching_rule_for_key(&self.profile.allow, key))
            .collect();
        if !rule_match_keys.is_empty() && allow_matches.len() == rule_match_keys.len() {
            return PermissionDecision::new(
                PermissionDecisionKind::Allow,
                audit_key,
                "matched allow rule",
                allow_matches.first().map(|rule| (*rule).clone()),
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
                format!("no allow rule matched '{}'", audit_key),
                None,
            ),
            PermissionBehavior::Ask => PermissionDecision::new(
                PermissionDecisionKind::Ask,
                audit_key.clone(),
                format!("No permission rule matched '{}'.", audit_key),
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
                .any(|key| PermissionCache::pattern_matches(&rule.pattern, key))
        })
    }

    fn matching_rule_for_key<'a>(
        &'a self,
        rules: &'a [PermissionRule],
        key: &str,
    ) -> Option<&'a PermissionRule> {
        rules
            .iter()
            .find(|rule| PermissionCache::pattern_matches(&rule.pattern, key))
    }

    fn path_is_in_workspace(&self, path: &str, working_directory: Option<&str>) -> bool {
        let path = normalize_path(path, working_directory);
        self.profile
            .filesystem
            .workspace_roots
            .iter()
            .map(|root| normalize_path(root, None))
            .any(|root| path_is_at_or_under(&path, &root))
    }

    fn path_is_protected(&self, path: &str, working_directory: Option<&str>) -> bool {
        let path = normalize_path(path, working_directory);
        self.profile
            .filesystem
            .protected_paths
            .iter()
            .any(|protected| {
                if Path::new(protected).is_absolute() {
                    return path_is_at_or_under(&path, &normalize_path(protected, None));
                }

                self.profile
                    .filesystem
                    .workspace_roots
                    .iter()
                    .map(|root| normalize_path(normalize_path(root, None).join(protected), None))
                    .any(|protected_path| path_is_at_or_under(&path, &protected_path))
            })
    }
}

fn path_is_at_or_under(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn normalize_path(path: impl AsRef<Path>, working_directory: Option<&str>) -> PathBuf {
    let path = path.as_ref();
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(working_directory) = working_directory {
        normalize_path(working_directory, None).join(path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    canonicalize_existing_components(&absolute)
}

fn canonicalize_existing_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }

        if let Ok(canonical) = normalized.canonicalize() {
            normalized = canonical;
        }
    }

    normalized
}

#[cfg(test)]
#[path = "decision_engine_tests.rs"]
mod tests;
