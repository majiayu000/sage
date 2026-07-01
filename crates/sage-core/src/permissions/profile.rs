use crate::settings::types::{PermissionSettings, SettingsPermissionBehavior};
use serde::{Deserialize, Serialize};

/// Source precedence for permission profile fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionProfileSource {
    System,
    User,
    Project,
    Local,
    Runtime,
}

impl PermissionProfileSource {
    fn precedence(self) -> u8 {
        match self {
            Self::System => 0,
            Self::User => 10,
            Self::Project => 20,
            Self::Local => 30,
            Self::Runtime => 40,
        }
    }
}

impl Default for PermissionProfileSource {
    fn default() -> Self {
        Self::Runtime
    }
}

/// Rule behavior used by the central permission engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

impl Default for PermissionBehavior {
    fn default() -> Self {
        Self::Ask
    }
}

impl From<SettingsPermissionBehavior> for PermissionBehavior {
    fn from(value: SettingsPermissionBehavior) -> Self {
        match value {
            SettingsPermissionBehavior::Allow => Self::Allow,
            SettingsPermissionBehavior::Deny => Self::Deny,
            SettingsPermissionBehavior::Ask => Self::Ask,
        }
    }
}

/// A single permission rule with its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRule {
    pub pattern: String,
    pub source: PermissionProfileSource,
}

impl PermissionRule {
    pub fn new(pattern: impl Into<String>, source: PermissionProfileSource) -> Self {
        Self {
            pattern: pattern.into(),
            source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemPermissionProfile {
    pub workspace_roots: Vec<String>,
    pub allow_outside_workspace: bool,
    pub protected_paths: Vec<String>,
}

impl Default for FilesystemPermissionProfile {
    fn default() -> Self {
        Self {
            workspace_roots: Vec::new(),
            allow_outside_workspace: false,
            protected_paths: vec![".git".to_string(), ".sage".to_string(), ".ssh".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPermissionProfile {
    pub enabled: bool,
}

impl Default for NetworkPermissionProfile {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecPermissionProfile {
    pub enabled: bool,
}

impl Default for ExecPermissionProfile {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SandboxPermissionProfile {
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ApprovalPermissionProfile {
    pub prompt_timeout_ms: Option<u64>,
    pub cache_ttl_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PermissionDomainSources {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<PermissionProfileSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<PermissionProfileSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec: Option<PermissionProfileSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<PermissionProfileSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<PermissionProfileSource>,
}

impl PermissionDomainSources {
    fn is_empty(&self) -> bool {
        self.filesystem.is_none()
            && self.network.is_none()
            && self.exec.is_none()
            && self.sandbox.is_none()
            && self.approval.is_none()
    }
}

/// Unified runtime permission profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionProfile {
    pub source: PermissionProfileSource,
    pub filesystem: FilesystemPermissionProfile,
    pub network: NetworkPermissionProfile,
    pub exec: ExecPermissionProfile,
    pub sandbox: SandboxPermissionProfile,
    pub approval: ApprovalPermissionProfile,
    pub allow: Vec<PermissionRule>,
    pub deny: Vec<PermissionRule>,
    pub default_behavior: PermissionBehavior,
    pub default_behavior_set: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_behavior_source: Option<PermissionProfileSource>,
    #[serde(default, skip_serializing_if = "PermissionDomainSources::is_empty")]
    pub domain_sources: PermissionDomainSources,
}

impl Default for PermissionProfile {
    fn default() -> Self {
        Self {
            source: PermissionProfileSource::Runtime,
            filesystem: FilesystemPermissionProfile::default(),
            network: NetworkPermissionProfile::default(),
            exec: ExecPermissionProfile::default(),
            sandbox: SandboxPermissionProfile::default(),
            approval: ApprovalPermissionProfile::default(),
            allow: Vec::new(),
            deny: Vec::new(),
            default_behavior: PermissionBehavior::Ask,
            default_behavior_set: false,
            default_behavior_source: None,
            domain_sources: PermissionDomainSources::default(),
        }
    }
}

impl PermissionProfile {
    pub fn with_source(mut self, source: PermissionProfileSource) -> Self {
        self.source = source;
        self
    }

    pub fn add_allow(
        mut self,
        pattern: impl Into<String>,
        source: PermissionProfileSource,
    ) -> Self {
        self.allow.push(PermissionRule::new(pattern, source));
        self
    }

    pub fn add_deny(mut self, pattern: impl Into<String>, source: PermissionProfileSource) -> Self {
        self.deny.push(PermissionRule::new(pattern, source));
        self
    }

    pub fn with_default_behavior(mut self, behavior: PermissionBehavior) -> Self {
        self.default_behavior = behavior;
        self.default_behavior_set = true;
        self.default_behavior_source = Some(self.source);
        self
    }

    pub fn with_filesystem_profile(
        mut self,
        filesystem: FilesystemPermissionProfile,
        source: PermissionProfileSource,
    ) -> Self {
        self.filesystem = filesystem;
        self.domain_sources.filesystem = Some(source);
        self
    }

    pub fn with_network_profile(
        mut self,
        network: NetworkPermissionProfile,
        source: PermissionProfileSource,
    ) -> Self {
        self.network = network;
        self.domain_sources.network = Some(source);
        self
    }

    pub fn with_exec_profile(
        mut self,
        exec: ExecPermissionProfile,
        source: PermissionProfileSource,
    ) -> Self {
        self.exec = exec;
        self.domain_sources.exec = Some(source);
        self
    }

    pub fn with_sandbox_profile(
        mut self,
        sandbox: SandboxPermissionProfile,
        source: PermissionProfileSource,
    ) -> Self {
        self.sandbox = sandbox;
        self.domain_sources.sandbox = Some(source);
        self
    }

    pub fn with_approval_profile(
        mut self,
        approval: ApprovalPermissionProfile,
        source: PermissionProfileSource,
    ) -> Self {
        self.approval = approval;
        self.domain_sources.approval = Some(source);
        self
    }

    pub fn has_configured_rules(&self) -> bool {
        !self.allow.is_empty()
            || !self.deny.is_empty()
            || self.default_behavior_set
            || self.default_behavior != PermissionBehavior::Ask
    }

    pub fn merge(&mut self, other: PermissionProfile) {
        self.allow.extend(other.allow);
        self.deny.extend(other.deny);

        if Self::source_overrides(
            self.domain_sources.filesystem,
            other.domain_sources.filesystem,
        ) {
            self.filesystem = other.filesystem;
            self.domain_sources.filesystem = other.domain_sources.filesystem;
        }
        if Self::source_overrides(self.domain_sources.network, other.domain_sources.network) {
            self.network = other.network;
            self.domain_sources.network = other.domain_sources.network;
        }
        if Self::source_overrides(self.domain_sources.exec, other.domain_sources.exec) {
            self.exec = other.exec;
            self.domain_sources.exec = other.domain_sources.exec;
        }
        if Self::source_overrides(self.domain_sources.sandbox, other.domain_sources.sandbox) {
            self.sandbox = other.sandbox;
            self.domain_sources.sandbox = other.domain_sources.sandbox;
        }
        if Self::source_overrides(self.domain_sources.approval, other.domain_sources.approval) {
            self.approval = other.approval;
            self.domain_sources.approval = other.domain_sources.approval;
        }

        if other.source.precedence() >= self.source.precedence() {
            self.source = other.source;
        }

        let other_default_source = other.default_behavior_source.or_else(|| {
            (other.default_behavior_set || other.default_behavior != PermissionBehavior::Ask)
                .then_some(other.source)
        });
        if Self::source_overrides(self.default_behavior_source, other_default_source) {
            self.default_behavior = other.default_behavior;
            self.default_behavior_set = true;
            self.default_behavior_source = other_default_source;
        }
    }

    fn source_overrides(
        current: Option<PermissionProfileSource>,
        incoming: Option<PermissionProfileSource>,
    ) -> bool {
        let Some(incoming) = incoming else {
            return false;
        };
        match current {
            Some(current) => incoming.precedence() >= current.precedence(),
            None => true,
        }
    }

    pub(crate) fn from_settings(settings: &PermissionSettings) -> Self {
        let default_behavior_source = (settings.default_behavior_set
            || settings.default_behavior != SettingsPermissionBehavior::Ask)
            .then_some(PermissionProfileSource::Local);
        Self {
            source: PermissionProfileSource::Local,
            allow: settings
                .allow
                .iter()
                .cloned()
                .map(|pattern| PermissionRule::new(pattern, PermissionProfileSource::Local))
                .collect(),
            deny: settings
                .deny
                .iter()
                .cloned()
                .map(|pattern| PermissionRule::new(pattern, PermissionProfileSource::Local))
                .collect(),
            default_behavior: settings.default_behavior.into(),
            default_behavior_set: settings.default_behavior_set,
            default_behavior_source,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_keeps_rules_and_higher_precedence_domains() {
        let mut base = PermissionProfile::default()
            .with_source(PermissionProfileSource::User)
            .with_network_profile(
                NetworkPermissionProfile { enabled: false },
                PermissionProfileSource::User,
            )
            .add_allow("Read(src/**)", PermissionProfileSource::User);

        let local = PermissionProfile::default()
            .with_source(PermissionProfileSource::Local)
            .with_network_profile(
                NetworkPermissionProfile { enabled: true },
                PermissionProfileSource::Local,
            )
            .add_deny("Read(secrets/**)", PermissionProfileSource::Local)
            .with_default_behavior(PermissionBehavior::Deny);

        base.merge(local);

        assert_eq!(base.allow.len(), 1);
        assert_eq!(base.deny.len(), 1);
        assert!(base.network.enabled);
        assert_eq!(base.default_behavior, PermissionBehavior::Deny);
        assert!(base.default_behavior_set);
    }

    #[test]
    fn lower_precedence_profile_cannot_downgrade_domains() {
        let mut runtime = PermissionProfile::default()
            .with_source(PermissionProfileSource::Runtime)
            .with_exec_profile(
                ExecPermissionProfile { enabled: false },
                PermissionProfileSource::Runtime,
            );

        let user = PermissionProfile::default()
            .with_source(PermissionProfileSource::User)
            .with_exec_profile(
                ExecPermissionProfile { enabled: true },
                PermissionProfileSource::User,
            );

        runtime.merge(user);

        assert!(!runtime.exec.enabled);
    }

    #[test]
    fn settings_fragment_does_not_override_domains_or_higher_default() {
        let mut runtime = PermissionProfile::default()
            .with_source(PermissionProfileSource::Runtime)
            .with_filesystem_profile(
                FilesystemPermissionProfile {
                    workspace_roots: vec!["/repo".to_string()],
                    ..Default::default()
                },
                PermissionProfileSource::Runtime,
            )
            .with_network_profile(
                NetworkPermissionProfile { enabled: false },
                PermissionProfileSource::Runtime,
            )
            .with_default_behavior(PermissionBehavior::Deny);
        let settings = PermissionSettings {
            allow: vec!["Bash(echo *)".to_string()],
            default_behavior: SettingsPermissionBehavior::Allow,
            default_behavior_set: true,
            ..Default::default()
        };

        runtime.merge(PermissionProfile::from_settings(&settings));

        assert_eq!(runtime.filesystem.workspace_roots, vec!["/repo"]);
        assert!(!runtime.network.enabled);
        assert_eq!(runtime.default_behavior, PermissionBehavior::Deny);
        assert_eq!(runtime.allow.len(), 1);
    }
}
