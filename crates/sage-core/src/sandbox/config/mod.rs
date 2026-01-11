//! Sandbox configuration following Claude Code patterns.

mod defaults;
mod mode;

pub use mode::{SandboxMode, ValidationStrictness};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use super::limits::ResourceLimits;
use defaults::{
    always_blocked_commands, default_allowed_commands, default_blocked_commands,
    default_env_passthrough, strict_allowed_commands,
};

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Whether sandbox is enabled
    pub enabled: bool,

    /// Sandbox operation mode
    pub mode: SandboxMode,

    /// Working directory for sandboxed operations
    pub working_dir: Option<PathBuf>,

    /// Paths allowed for reading
    pub allowed_read_paths: Vec<PathBuf>,

    /// Paths allowed for writing
    pub allowed_write_paths: Vec<PathBuf>,

    /// Commands allowed to execute
    pub allowed_commands: Vec<String>,

    /// Commands explicitly blocked
    pub blocked_commands: Vec<String>,

    /// Resource limits
    pub limits: ResourceLimits,

    /// Execution timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Allow network access
    pub allow_network: bool,

    /// Allowed network hosts (if allow_network is true)
    pub allowed_hosts: Vec<String>,

    /// Blocked network hosts
    pub blocked_hosts: Vec<String>,

    /// Environment variables to pass through
    pub env_passthrough: Vec<String>,

    /// Environment variables to set
    pub env_override: Vec<(String, String)>,

    // === Claude Code style security options ===
    /// Validation strictness level for command checks
    #[serde(default)]
    pub validation_strictness: ValidationStrictness,

    /// Whether to track violations
    #[serde(default = "default_true")]
    pub track_violations: bool,

    /// Maximum number of violations to store
    #[serde(default = "default_max_violations")]
    pub max_violations: usize,

    /// Whether to annotate stderr with violation info
    #[serde(default = "default_true")]
    pub annotate_stderr: bool,

    /// Additional sensitive file patterns to protect
    #[serde(default)]
    pub additional_sensitive_files: Vec<String>,

    /// Additional allowed tmp paths (beyond /tmp/sage/)
    #[serde(default)]
    pub allowed_tmp_paths: Vec<PathBuf>,

    /// Whether to enforce strict sensitive file protection
    #[serde(default = "default_true")]
    pub strict_sensitive_files: bool,
}

fn default_true() -> bool {
    true
}

fn default_max_violations() -> usize {
    1000
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: SandboxMode::Restricted,
            working_dir: None,
            allowed_read_paths: vec![],
            allowed_write_paths: vec![],
            allowed_commands: default_allowed_commands(),
            blocked_commands: default_blocked_commands(),
            limits: ResourceLimits::default(),
            timeout: Duration::from_secs(120),
            allow_network: true,
            allowed_hosts: vec![],
            blocked_hosts: vec![],
            env_passthrough: default_env_passthrough(),
            env_override: vec![],
            // Claude Code style security defaults
            validation_strictness: ValidationStrictness::Standard,
            track_violations: true,
            max_violations: 1000,
            annotate_stderr: true,
            additional_sensitive_files: vec![],
            allowed_tmp_paths: vec![],
            strict_sensitive_files: true,
        }
    }
}

impl SandboxConfig {
    /// Create a permissive configuration (minimal restrictions)
    pub fn permissive() -> Self {
        Self {
            enabled: true,
            mode: SandboxMode::Permissive,
            working_dir: None,
            allowed_read_paths: vec![PathBuf::from("/")],
            allowed_write_paths: vec![],
            allowed_commands: vec![], // Empty means all allowed
            blocked_commands: always_blocked_commands(),
            limits: ResourceLimits::permissive(),
            timeout: Duration::from_secs(300),
            allow_network: true,
            allowed_hosts: vec![],
            blocked_hosts: vec![],
            env_passthrough: vec!["*".to_string()],
            env_override: vec![],
            // Minimal validation in permissive mode
            validation_strictness: ValidationStrictness::Minimal,
            track_violations: true,
            max_violations: 1000,
            annotate_stderr: false,
            additional_sensitive_files: vec![],
            allowed_tmp_paths: vec![],
            strict_sensitive_files: false,
        }
    }

    /// Create a strict configuration (maximum restrictions)
    pub fn strict(working_dir: PathBuf) -> Self {
        Self {
            enabled: true,
            mode: SandboxMode::Strict,
            working_dir: Some(working_dir.clone()),
            allowed_read_paths: vec![working_dir.clone()],
            allowed_write_paths: vec![working_dir],
            allowed_commands: strict_allowed_commands(),
            blocked_commands: default_blocked_commands(),
            limits: ResourceLimits::strict(),
            timeout: Duration::from_secs(30),
            allow_network: false,
            allowed_hosts: vec![],
            blocked_hosts: vec![],
            env_passthrough: vec![],
            env_override: vec![],
            // Maximum validation in strict mode
            validation_strictness: ValidationStrictness::Strict,
            track_violations: true,
            max_violations: 1000,
            annotate_stderr: true,
            additional_sensitive_files: vec![],
            allowed_tmp_paths: vec![],
            strict_sensitive_files: true,
        }
    }

    /// Get a ValidationContext based on this config's strictness
    pub fn to_validation_context(&self) -> super::validation::ValidationContext {
        super::validation::ValidationContext {
            allow_chaining: self.validation_strictness.allows_chaining(),
            allow_background: self.validation_strictness.allows_background(),
            working_directory: self.working_dir.as_ref().map(|p| p.display().to_string()),
            dangerous_commands: self.blocked_commands.clone(),
        }
    }

    /// Check if a command is allowed
    pub fn is_command_allowed(&self, command: &str) -> bool {
        // Extract the base command (first word)
        let base_command = command.split_whitespace().next().unwrap_or(command);
        let base_command = std::path::Path::new(base_command)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(base_command);

        // Always check blocked commands first
        if self.blocked_commands.iter().any(|c| c == base_command) {
            return false;
        }

        // If allowed_commands is empty, all non-blocked commands are allowed
        if self.allowed_commands.is_empty() {
            return true;
        }

        // Check if command is in allowed list
        self.allowed_commands.iter().any(|c| c == base_command)
    }

    /// Check if a path is readable
    pub fn is_path_readable(&self, path: &PathBuf) -> bool {
        // In permissive mode with "/" allowed, everything is readable
        if self.allowed_read_paths.iter().any(|p| p.as_os_str() == "/") {
            return true;
        }

        // Check if path is under any allowed read path
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        self.allowed_read_paths.iter().any(|allowed| {
            let allowed_canonical = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            canonical.starts_with(&allowed_canonical)
        })
    }

    /// Check if a path is writable
    pub fn is_path_writable(&self, path: &PathBuf) -> bool {
        // Check if path is under any allowed write path
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        self.allowed_write_paths.iter().any(|allowed| {
            let allowed_canonical = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            canonical.starts_with(&allowed_canonical)
        })
    }

    /// Check if a host is allowed for network access
    pub fn is_host_allowed(&self, host: &str) -> bool {
        if !self.allow_network {
            return false;
        }

        // Check blocked hosts first
        if self.blocked_hosts.iter().any(|h| host.contains(h)) {
            return false;
        }

        // If allowed_hosts is empty, all non-blocked hosts are allowed
        if self.allowed_hosts.is_empty() {
            return true;
        }

        // Check if host is in allowed list
        self.allowed_hosts.iter().any(|h| host.contains(h))
    }
}

#[cfg(test)]
mod tests;
