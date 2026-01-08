//! Types for OS-level sandbox configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result type for OS sandbox operations
pub type OsSandboxResult<T> = Result<T, crate::sandbox::SandboxError>;

/// OS-level sandbox mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsSandboxMode {
    /// No OS-level sandbox (only policy-based restrictions)
    #[default]
    Disabled,
    /// Read-only mode - allow reading but not writing
    ReadOnly,
    /// Network restricted - deny network access
    NoNetwork,
    /// Strict mode - minimal permissions
    Strict,
    /// Custom profile (platform-specific)
    Custom,
}

impl OsSandboxMode {
    /// Check if the mode is enabled
    pub fn is_enabled(&self) -> bool {
        !matches!(self, OsSandboxMode::Disabled)
    }
}

/// Configuration for OS-level sandbox
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsSandboxConfig {
    /// Sandbox mode
    pub mode: OsSandboxMode,

    /// Working directory (allowed read/write)
    pub working_dir: Option<PathBuf>,

    /// Additional read-only paths
    #[serde(default)]
    pub read_only_paths: Vec<PathBuf>,

    /// Additional write paths
    #[serde(default)]
    pub write_paths: Vec<PathBuf>,

    /// Allow network access
    #[serde(default)]
    pub allow_network: bool,

    /// Allow process execution
    #[serde(default = "default_allow_process")]
    pub allow_process: bool,

    /// Allow system-wide temporary directory
    #[serde(default = "default_allow_tmp")]
    pub allow_tmp: bool,

    /// Custom sandbox profile (platform-specific)
    ///
    /// On macOS: Path to a .sb file or inline SBPL code
    /// On Linux: Path to seccomp filter or inline BPF
    pub custom_profile: Option<String>,

    /// Environment variables allowed to pass through
    #[serde(default)]
    pub allowed_env_vars: Vec<String>,
}

fn default_allow_process() -> bool {
    true
}

fn default_allow_tmp() -> bool {
    true
}

impl OsSandboxConfig {
    /// Create a new config with the given mode
    pub fn new(mode: OsSandboxMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    /// Create a disabled sandbox config
    pub fn disabled() -> Self {
        Self::new(OsSandboxMode::Disabled)
    }

    /// Create a read-only sandbox config
    pub fn read_only(working_dir: PathBuf) -> Self {
        Self {
            mode: OsSandboxMode::ReadOnly,
            working_dir: Some(working_dir),
            allow_tmp: true,
            allow_process: true,
            ..Default::default()
        }
    }

    /// Create a network-restricted sandbox config
    pub fn no_network(working_dir: PathBuf) -> Self {
        Self {
            mode: OsSandboxMode::NoNetwork,
            working_dir: Some(working_dir),
            allow_network: false,
            allow_tmp: true,
            allow_process: true,
            ..Default::default()
        }
    }

    /// Create a strict sandbox config
    pub fn strict(working_dir: PathBuf) -> Self {
        Self {
            mode: OsSandboxMode::Strict,
            working_dir: Some(working_dir),
            allow_network: false,
            allow_tmp: false,
            allow_process: false,
            ..Default::default()
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, path: PathBuf) -> Self {
        self.working_dir = Some(path);
        self
    }

    /// Add a read-only path
    pub fn with_read_only_path(mut self, path: PathBuf) -> Self {
        self.read_only_paths.push(path);
        self
    }

    /// Add a write path
    pub fn with_write_path(mut self, path: PathBuf) -> Self {
        self.write_paths.push(path);
        self
    }

    /// Set network access
    pub fn with_network(mut self, allow: bool) -> Self {
        self.allow_network = allow;
        self
    }

    /// Set process execution
    pub fn with_process(mut self, allow: bool) -> Self {
        self.allow_process = allow;
        self
    }

    /// Set custom profile
    pub fn with_custom_profile(mut self, profile: impl Into<String>) -> Self {
        self.custom_profile = Some(profile.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_sandbox_mode_default() {
        let mode = OsSandboxMode::default();
        assert!(!mode.is_enabled());
    }

    #[test]
    fn test_os_sandbox_mode_enabled() {
        assert!(OsSandboxMode::ReadOnly.is_enabled());
        assert!(OsSandboxMode::NoNetwork.is_enabled());
        assert!(OsSandboxMode::Strict.is_enabled());
        assert!(OsSandboxMode::Custom.is_enabled());
    }

    #[test]
    fn test_os_sandbox_config_builder() {
        let config = OsSandboxConfig::new(OsSandboxMode::ReadOnly)
            .with_working_dir(PathBuf::from("/tmp/sandbox"))
            .with_read_only_path(PathBuf::from("/usr"))
            .with_network(false);

        assert_eq!(config.mode, OsSandboxMode::ReadOnly);
        assert!(config.working_dir.is_some());
        assert_eq!(config.read_only_paths.len(), 1);
        assert!(!config.allow_network);
    }

    #[test]
    fn test_os_sandbox_config_strict() {
        let config = OsSandboxConfig::strict(PathBuf::from("/tmp"));
        assert_eq!(config.mode, OsSandboxMode::Strict);
        assert!(!config.allow_network);
        assert!(!config.allow_tmp);
        assert!(!config.allow_process);
    }
}
