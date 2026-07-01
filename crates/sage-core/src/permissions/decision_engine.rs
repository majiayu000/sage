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
            .unwrap_or_else(|| self.tool_name.clone())
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

        if let Some(path) = input.path.as_deref() {
            if self.path_is_protected(path) {
                return PermissionDecision::new(
                    PermissionDecisionKind::Deny,
                    audit_key,
                    format!("path '{}' is protected by permission profile", path),
                    None,
                );
            }

            if !self.profile.filesystem.allow_outside_workspace
                && !self.profile.filesystem.workspace_roots.is_empty()
                && !self.path_is_in_workspace(path)
            {
                return PermissionDecision::new(
                    PermissionDecisionKind::Deny,
                    audit_key,
                    format!("path '{}' is outside configured workspace roots", path),
                    None,
                );
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

        if let Some(rule) = self.matching_rule(&self.profile.deny, &input.permission_keys) {
            return PermissionDecision::new(
                PermissionDecisionKind::Deny,
                audit_key,
                format!("matched deny rule '{}'", rule.pattern),
                Some(rule.clone()),
            );
        }

        let allow_matches: Vec<&PermissionRule> = input
            .permission_keys
            .iter()
            .filter_map(|key| self.matching_rule_for_key(&self.profile.allow, key))
            .collect();
        if !input.permission_keys.is_empty() && allow_matches.len() == input.permission_keys.len() {
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

    fn path_is_in_workspace(&self, path: &str) -> bool {
        let path = normalize_path(path);
        self.profile
            .filesystem
            .workspace_roots
            .iter()
            .map(|root| normalize_path(root))
            .any(|root| path_is_at_or_under(&path, &root))
    }

    fn path_is_protected(&self, path: &str) -> bool {
        let path = normalize_path(path);
        self.profile
            .filesystem
            .protected_paths
            .iter()
            .any(|protected| {
                if protected.starts_with('/') {
                    return path_is_at_or_under(&path, &normalize_path(protected));
                }

                self.profile
                    .filesystem
                    .workspace_roots
                    .iter()
                    .map(|root| normalize_path(normalize_path(root).join(protected)))
                    .any(|protected_path| path_is_at_or_under(&path, &protected_path))
            })
    }
}

fn path_is_at_or_under(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let absolute = if path.is_absolute() {
        path.to_path_buf()
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
