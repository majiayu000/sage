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

        if other.source.precedence() >= self.source.precedence() {
            self.filesystem = other.filesystem;
            self.network = other.network;
            self.exec = other.exec;
            self.sandbox = other.sandbox;
            self.approval = other.approval;
            self.source = other.source;
        }

        if other.default_behavior_set || other.default_behavior != PermissionBehavior::Ask {
            self.default_behavior = other.default_behavior;
            self.default_behavior_set = true;
        }
    }

    pub(crate) fn from_settings(settings: &PermissionSettings) -> Self {
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
            .add_allow("Read(src/**)", PermissionProfileSource::User);
        base.network.enabled = false;

        let mut local = PermissionProfile::default()
            .with_source(PermissionProfileSource::Local)
            .add_deny("Read(secrets/**)", PermissionProfileSource::Local)
            .with_default_behavior(PermissionBehavior::Deny);
        local.network.enabled = true;

        base.merge(local);

        assert_eq!(base.allow.len(), 1);
        assert_eq!(base.deny.len(), 1);
        assert!(base.network.enabled);
        assert_eq!(base.default_behavior, PermissionBehavior::Deny);
        assert!(base.default_behavior_set);
    }

    #[test]
    fn lower_precedence_profile_cannot_downgrade_domains() {
        let mut runtime =
            PermissionProfile::default().with_source(PermissionProfileSource::Runtime);
        runtime.exec.enabled = false;

        let mut user = PermissionProfile::default().with_source(PermissionProfileSource::User);
        user.exec.enabled = true;

        runtime.merge(user);

        assert!(!runtime.exec.enabled);
    }
}
