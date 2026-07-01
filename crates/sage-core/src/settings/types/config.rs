//! UI, Workspace, Model, and managed policy settings.

use crate::error::{SageError, SageResult};
use crate::hashing::bytes_to_hex;
use crate::permissions::{
    ExecPermissionProfile, NetworkPermissionProfile, PermissionBehavior, PermissionProfile,
    PermissionProfileSource, SandboxPermissionProfile,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// UI settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiSettings {
    /// Show progress indicators
    #[serde(default)]
    pub show_progress: Option<bool>,

    /// Theme (light/dark/auto)
    #[serde(default)]
    pub theme: Option<String>,

    /// Enable colors
    #[serde(default)]
    pub colors: Option<bool>,

    /// Verbose output
    #[serde(default)]
    pub verbose: Option<bool>,

    /// Maximum output width
    #[serde(default)]
    pub max_width: Option<usize>,
}

impl UiSettings {
    /// Merge another UI settings
    pub fn merge(&mut self, other: UiSettings) {
        if other.show_progress.is_some() {
            self.show_progress = other.show_progress;
        }
        if other.theme.is_some() {
            self.theme = other.theme;
        }
        if other.colors.is_some() {
            self.colors = other.colors;
        }
        if other.verbose.is_some() {
            self.verbose = other.verbose;
        }
        if other.max_width.is_some() {
            self.max_width = other.max_width;
        }
    }
}

/// Workspace settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// Files/directories to ignore
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Include patterns
    #[serde(default)]
    pub include: Vec<String>,

    /// Working directory override
    #[serde(default)]
    pub working_directory: Option<String>,

    /// Project type hint
    #[serde(default)]
    pub project_type: Option<String>,
}

impl WorkspaceSettings {
    /// Merge another workspace settings
    pub fn merge(&mut self, other: WorkspaceSettings) {
        self.ignore.extend(other.ignore);
        self.include.extend(other.include);
        if other.working_directory.is_some() {
            self.working_directory = other.working_directory;
        }
        if other.project_type.is_some() {
            self.project_type = other.project_type;
        }
    }
}

/// Model settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelSettings {
    /// Default model to use
    #[serde(default)]
    pub default_model: Option<String>,

    /// Maximum tokens
    #[serde(default)]
    pub max_tokens: Option<usize>,

    /// Temperature
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Provider override
    #[serde(default)]
    pub provider: Option<String>,

    /// API base URL override
    #[serde(default)]
    pub api_base: Option<String>,
}

impl ModelSettings {
    /// Merge another model settings
    pub fn merge(&mut self, other: ModelSettings) {
        if other.default_model.is_some() {
            self.default_model = other.default_model;
        }
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        if other.temperature.is_some() {
            self.temperature = other.temperature;
        }
        if other.provider.is_some() {
            self.provider = other.provider;
        }
        if other.api_base.is_some() {
            self.api_base = other.api_base;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedConfigSourceKind {
    Organization,
    Team,
    ProjectPolicy,
}

impl Default for ManagedConfigSourceKind {
    fn default() -> Self {
        Self::ProjectPolicy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedConfigSource {
    pub kind: ManagedConfigSourceKind,
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedConfig {
    #[serde(default)]
    pub source_kind: ManagedConfigSourceKind,
    #[serde(default)]
    pub permissions: ManagedPermissionConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedPermissionConfig {
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub protected_paths: Vec<String>,
    #[serde(default)]
    pub default_behavior: Option<ManagedDefaultBehavior>,
    #[serde(default)]
    pub network: Option<ManagedNetworkConfig>,
    #[serde(default)]
    pub exec: Option<ManagedExecConfig>,
    #[serde(default)]
    pub sandbox: Option<ManagedSandboxConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagedDefaultBehavior {
    Ask,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedNetworkConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedExecConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedSandboxConfig {
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedManagedConfig {
    pub config: ManagedConfig,
    pub source: ManagedConfigSource,
}

impl ManagedConfig {
    pub fn parse_json(content: &str) -> SageResult<Self> {
        let config: Self = serde_json::from_str(content).map_err(|error| {
            SageError::config(format!("Failed to parse managed config: {error}"))
        })?;
        config.validate_restrictive_only()?;
        Ok(config)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> SageResult<LoadedManagedConfig> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|error| {
            SageError::io_with_path(error.to_string(), path.display().to_string())
        })?;
        let config = Self::parse_json(&content)?;
        let sha256 = bytes_to_hex(Sha256::digest(content.as_bytes()));
        let source = ManagedConfigSource {
            kind: config.source_kind.clone(),
            path: path.to_path_buf(),
            sha256,
        };
        Ok(LoadedManagedConfig { config, source })
    }

    pub fn validate_restrictive_only(&self) -> SageResult<()> {
        if let Some(network) = self.permissions.network
            && network.enabled
        {
            return Err(SageError::config(
                "managed config cannot enable network access",
            ));
        }
        if let Some(exec) = self.permissions.exec
            && exec.enabled
        {
            return Err(SageError::config(
                "managed config cannot enable process execution",
            ));
        }
        if let Some(sandbox) = self.permissions.sandbox
            && !sandbox.required
        {
            return Err(SageError::config(
                "managed config cannot disable required sandbox policy",
            ));
        }
        Ok(())
    }

    pub fn to_permission_profile(&self) -> PermissionProfile {
        let mut profile =
            PermissionProfile::default().with_source(PermissionProfileSource::Managed);
        for deny in &self.permissions.deny {
            profile = profile.add_deny(deny.clone(), PermissionProfileSource::Managed);
        }
        if let Some(default_behavior) = self.permissions.default_behavior {
            profile = profile.with_default_behavior(match default_behavior {
                ManagedDefaultBehavior::Ask => PermissionBehavior::Ask,
                ManagedDefaultBehavior::Deny => PermissionBehavior::Deny,
            });
        }
        if !self.permissions.protected_paths.is_empty() {
            let mut filesystem = profile.filesystem.clone();
            filesystem
                .protected_paths
                .extend(self.permissions.protected_paths.clone());
            filesystem.protected_paths.sort();
            filesystem.protected_paths.dedup();
            profile = profile.with_filesystem_profile(filesystem, PermissionProfileSource::Managed);
        }
        if let Some(network) = self.permissions.network {
            profile = profile.with_network_profile(
                NetworkPermissionProfile {
                    enabled: network.enabled,
                },
                PermissionProfileSource::Managed,
            );
        }
        if let Some(exec) = self.permissions.exec {
            profile = profile.with_exec_profile(
                ExecPermissionProfile {
                    enabled: exec.enabled,
                },
                PermissionProfileSource::Managed,
            );
        }
        if let Some(sandbox) = self.permissions.sandbox {
            profile = profile.with_sandbox_profile(
                SandboxPermissionProfile {
                    required: sandbox.required,
                },
                PermissionProfileSource::Managed,
            );
        }
        profile
    }

    pub fn apply_restrictive_to(&self, target: &mut PermissionProfile) {
        let managed = self.to_permission_profile();
        target.deny.extend(managed.deny);
        if managed.default_behavior_set {
            target.default_behavior = match (target.default_behavior, managed.default_behavior) {
                (PermissionBehavior::Deny, _) | (_, PermissionBehavior::Deny) => {
                    PermissionBehavior::Deny
                }
                _ => PermissionBehavior::Ask,
            };
            target.default_behavior_set = true;
            target.default_behavior_source = Some(PermissionProfileSource::Managed);
        }
        if managed.domain_sources.filesystem.is_some() {
            target
                .filesystem
                .protected_paths
                .extend(managed.filesystem.protected_paths);
            target.filesystem.protected_paths.sort();
            target.filesystem.protected_paths.dedup();
            target.domain_sources.filesystem = Some(PermissionProfileSource::Managed);
        }
        if managed.domain_sources.network.is_some() {
            target.network.enabled = target.network.enabled && managed.network.enabled;
            target.domain_sources.network = Some(PermissionProfileSource::Managed);
        }
        if managed.domain_sources.exec.is_some() {
            target.exec.enabled = target.exec.enabled && managed.exec.enabled;
            target.domain_sources.exec = Some(PermissionProfileSource::Managed);
        }
        if managed.domain_sources.sandbox.is_some() {
            target.sandbox.required = target.sandbox.required || managed.sandbox.required;
            target.domain_sources.sandbox = Some(PermissionProfileSource::Managed);
        }
    }
}

#[cfg(test)]
mod managed_config_tests {
    use super::*;

    #[test]
    fn managed_config_rejects_unknown_and_permissive_fields() {
        let unknown = r#"{"permissions":{"allow":["Bash(*)"]}}"#;
        let enable_network = r#"{"permissions":{"network":{"enabled":true}}}"#;
        let disable_sandbox = r#"{"permissions":{"sandbox":{"required":false}}}"#;

        assert!(ManagedConfig::parse_json(unknown).is_err());
        assert!(ManagedConfig::parse_json(enable_network).is_err());
        assert!(ManagedConfig::parse_json(disable_sandbox).is_err());
    }

    #[test]
    fn managed_config_builds_restrictive_profile_with_managed_provenance() {
        let config = match ManagedConfig::parse_json(
            r#"{
                "source_kind": "organization",
                "permissions": {
                    "deny": ["Bash(curl *)"],
                    "protected_paths": [".env.local"],
                    "default_behavior": "deny",
                    "network": {"enabled": false},
                    "exec": {"enabled": false},
                    "sandbox": {"required": true}
                }
            }"#,
        ) {
            Ok(config) => config,
            Err(error) => panic!("expected managed config to parse: {error}"),
        };

        let profile = config.to_permission_profile();

        assert_eq!(profile.source, PermissionProfileSource::Managed);
        assert_eq!(profile.deny[0].source, PermissionProfileSource::Managed);
        assert_eq!(profile.default_behavior, PermissionBehavior::Deny);
        assert!(!profile.network.enabled);
        assert!(!profile.exec.enabled);
        assert!(profile.sandbox.required);
        assert!(
            profile
                .filesystem
                .protected_paths
                .contains(&".env.local".to_string())
        );
    }

    #[test]
    fn managed_config_apply_only_restricts_existing_profile() {
        let config = match ManagedConfig::parse_json(
            r#"{
                "permissions": {
                    "default_behavior": "ask",
                    "network": {"enabled": false},
                    "sandbox": {"required": true}
                }
            }"#,
        ) {
            Ok(config) => config,
            Err(error) => panic!("expected managed config to parse: {error}"),
        };
        let mut profile = PermissionProfile::default()
            .with_default_behavior(PermissionBehavior::Allow)
            .with_network_profile(
                NetworkPermissionProfile { enabled: true },
                PermissionProfileSource::Runtime,
            );

        config.apply_restrictive_to(&mut profile);

        assert_eq!(profile.default_behavior, PermissionBehavior::Ask);
        assert!(!profile.network.enabled);
        assert!(profile.sandbox.required);
    }

    #[test]
    fn managed_config_standard_merge_is_not_overridden_by_user_profile() {
        let managed = match ManagedConfig::parse_json(
            r#"{
                "permissions": {
                    "network": {"enabled": false},
                    "exec": {"enabled": false}
                }
            }"#,
        ) {
            Ok(config) => config.to_permission_profile(),
            Err(error) => panic!("expected managed config to parse: {error}"),
        };
        let user = PermissionProfile::default()
            .with_source(PermissionProfileSource::User)
            .with_network_profile(
                NetworkPermissionProfile { enabled: true },
                PermissionProfileSource::User,
            )
            .with_exec_profile(
                ExecPermissionProfile { enabled: true },
                PermissionProfileSource::User,
            );
        let mut profile = managed;

        profile.merge(user);

        assert!(!profile.network.enabled);
        assert!(!profile.exec.enabled);
        assert_eq!(
            profile.domain_sources.network,
            Some(PermissionProfileSource::Managed)
        );
    }
}
