//! Sandbox configuration following Claude Code patterns.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use super::limits::ResourceLimits;

/// Sandbox operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxMode {
    /// Permissive mode - minimal restrictions
    Permissive,
    /// Restricted mode - moderate restrictions
    Restricted,
    /// Strict mode - maximum restrictions
    Strict,
    /// Custom mode - user-defined restrictions
    Custom,
}

impl Default for SandboxMode {
    fn default() -> Self {
        Self::Restricted
    }
}

/// Validation strictness level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ValidationStrictness {
    /// Minimal validation - only block critical issues
    Minimal,
    /// Standard validation - balanced security
    #[default]
    Standard,
    /// Strict validation - maximum security
    Strict,
}

impl ValidationStrictness {
    /// Check if chaining is allowed at this strictness level
    pub fn allows_chaining(&self) -> bool {
        match self {
            ValidationStrictness::Minimal => true,
            ValidationStrictness::Standard => true,
            ValidationStrictness::Strict => false,
        }
    }

    /// Check if background execution is allowed
    pub fn allows_background(&self) -> bool {
        match self {
            ValidationStrictness::Minimal => true,
            ValidationStrictness::Standard => true,
            ValidationStrictness::Strict => false,
        }
    }
}

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
            allowed_commands: Self::default_allowed_commands(),
            blocked_commands: Self::default_blocked_commands(),
            limits: ResourceLimits::default(),
            timeout: Duration::from_secs(120),
            allow_network: true,
            allowed_hosts: vec![],
            blocked_hosts: vec![],
            env_passthrough: Self::default_env_passthrough(),
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
            blocked_commands: Self::always_blocked_commands(),
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
            allowed_commands: Self::strict_allowed_commands(),
            blocked_commands: Self::default_blocked_commands(),
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

    /// Default allowed commands for restricted mode
    fn default_allowed_commands() -> Vec<String> {
        vec![
            // File operations
            "ls".to_string(),
            "cat".to_string(),
            "head".to_string(),
            "tail".to_string(),
            "find".to_string(),
            "grep".to_string(),
            "wc".to_string(),
            "file".to_string(),
            "stat".to_string(),
            // Directory operations
            "pwd".to_string(),
            "cd".to_string(),
            "mkdir".to_string(),
            // Text processing
            "sed".to_string(),
            "awk".to_string(),
            "sort".to_string(),
            "uniq".to_string(),
            "cut".to_string(),
            "tr".to_string(),
            "diff".to_string(),
            // Development tools
            "git".to_string(),
            "cargo".to_string(),
            "rustc".to_string(),
            "npm".to_string(),
            "node".to_string(),
            "python".to_string(),
            "python3".to_string(),
            "pip".to_string(),
            "pip3".to_string(),
            // Build tools
            "make".to_string(),
            "cmake".to_string(),
            // Archive tools
            "tar".to_string(),
            "zip".to_string(),
            "unzip".to_string(),
            "gzip".to_string(),
            "gunzip".to_string(),
            // Utilities
            "echo".to_string(),
            "date".to_string(),
            "which".to_string(),
            "env".to_string(),
            "true".to_string(),
            "false".to_string(),
            "test".to_string(),
            "[".to_string(),
        ]
    }

    /// Strictly allowed commands for strict mode
    fn strict_allowed_commands() -> Vec<String> {
        vec![
            "ls".to_string(),
            "cat".to_string(),
            "head".to_string(),
            "tail".to_string(),
            "grep".to_string(),
            "wc".to_string(),
            "pwd".to_string(),
            "echo".to_string(),
            "true".to_string(),
            "false".to_string(),
        ]
    }

    /// Default blocked commands
    fn default_blocked_commands() -> Vec<String> {
        vec![
            // Dangerous system commands
            "rm".to_string(),
            "rmdir".to_string(),
            "mv".to_string(),
            "cp".to_string(),
            // System modification
            "chmod".to_string(),
            "chown".to_string(),
            "chgrp".to_string(),
            // Process management
            "kill".to_string(),
            "killall".to_string(),
            "pkill".to_string(),
            // Package managers (system-level)
            "apt".to_string(),
            "apt-get".to_string(),
            "yum".to_string(),
            "dnf".to_string(),
            "brew".to_string(),
            // Sudo and privilege escalation
            "sudo".to_string(),
            "su".to_string(),
            "doas".to_string(),
            // Network tools (dangerous)
            "nc".to_string(),
            "netcat".to_string(),
            "ncat".to_string(),
            "telnet".to_string(),
            // Shells (prevent shell escape)
            "sh".to_string(),
            "bash".to_string(),
            "zsh".to_string(),
            "fish".to_string(),
            "csh".to_string(),
            "tcsh".to_string(),
            // Other dangerous
            "eval".to_string(),
            "exec".to_string(),
            "source".to_string(),
            ".".to_string(),
            "dd".to_string(),
            "mkfs".to_string(),
            "fdisk".to_string(),
            "parted".to_string(),
        ]
    }

    /// Commands that should always be blocked regardless of mode
    fn always_blocked_commands() -> Vec<String> {
        vec![
            "sudo".to_string(),
            "su".to_string(),
            "doas".to_string(),
            "dd".to_string(),
            "mkfs".to_string(),
            "fdisk".to_string(),
            "parted".to_string(),
        ]
    }

    /// Default environment variables to pass through
    fn default_env_passthrough() -> Vec<String> {
        vec![
            "PATH".to_string(),
            "HOME".to_string(),
            "USER".to_string(),
            "LANG".to_string(),
            "LC_ALL".to_string(),
            "TERM".to_string(),
            "SHELL".to_string(),
            "EDITOR".to_string(),
            "VISUAL".to_string(),
            // Development related
            "CARGO_HOME".to_string(),
            "RUSTUP_HOME".to_string(),
            "GOPATH".to_string(),
            "GOROOT".to_string(),
            "NODE_PATH".to_string(),
            "NPM_CONFIG_PREFIX".to_string(),
            "PYTHONPATH".to_string(),
            "VIRTUAL_ENV".to_string(),
        ]
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
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SandboxConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, SandboxMode::Restricted);
        assert!(config.allow_network);
    }

    #[test]
    fn test_permissive_config() {
        let config = SandboxConfig::permissive();
        assert_eq!(config.mode, SandboxMode::Permissive);
        assert!(config.timeout > Duration::from_secs(60));
    }

    #[test]
    fn test_strict_config() {
        let config = SandboxConfig::strict(PathBuf::from("/tmp/sandbox"));
        assert_eq!(config.mode, SandboxMode::Strict);
        assert!(!config.allow_network);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_command_allowed() {
        let config = SandboxConfig::default();

        // Allowed commands
        assert!(config.is_command_allowed("ls"));
        assert!(config.is_command_allowed("git"));
        assert!(config.is_command_allowed("cargo"));

        // Blocked commands
        assert!(!config.is_command_allowed("rm"));
        assert!(!config.is_command_allowed("sudo"));
        assert!(!config.is_command_allowed("kill"));
    }

    #[test]
    fn test_command_with_path() {
        let config = SandboxConfig::default();

        // Commands with full path
        assert!(config.is_command_allowed("/usr/bin/ls"));
        assert!(!config.is_command_allowed("/bin/rm"));
    }

    #[test]
    fn test_host_allowed() {
        let mut config = SandboxConfig::default();

        // All hosts allowed by default
        assert!(config.is_host_allowed("example.com"));

        // Block a host
        config.blocked_hosts.push("blocked.com".to_string());
        assert!(!config.is_host_allowed("blocked.com"));
        assert!(config.is_host_allowed("example.com"));

        // Disable network
        config.allow_network = false;
        assert!(!config.is_host_allowed("example.com"));
    }

    #[test]
    fn test_permissive_allows_more_commands() {
        let config = SandboxConfig::permissive();

        // In permissive mode, most commands are allowed
        assert!(config.is_command_allowed("ls"));
        assert!(config.is_command_allowed("grep"));

        // But always-blocked are still blocked
        assert!(!config.is_command_allowed("sudo"));
    }
}
