//! Path access policy following Claude Code patterns.
//!
//! Provides comprehensive path access control including:
//! - Sensitive file protection (.gitconfig, .ssh/, etc.)
//! - Temp path restrictions (/tmp only allowed in /tmp/sage/)
//! - System path protection

use super::super::config::SandboxConfig;
use super::super::SandboxError;
use std::path::{Path, PathBuf};

/// Sensitive files that require special protection
/// Following Claude Code patterns
const SENSITIVE_FILES: &[&str] = &[
    // Git configuration
    ".gitconfig",
    ".git/config",
    ".git/hooks/",
    // Shell configuration
    ".bashrc",
    ".bash_profile",
    ".bash_history",
    ".zshrc",
    ".zsh_history",
    ".profile",
    ".zprofile",
    // SSH and credentials
    ".ssh/",
    ".aws/",
    ".docker/",
    ".kube/",
    ".gnupg/",
    // Package manager credentials
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    // Environment and secrets
    ".env",
    ".env.local",
    ".env.production",
    "secrets.yaml",
    "secrets.json",
    ".secrets",
    // IDE and editor configs (may contain tokens)
    ".vscode/settings.json",
    ".idea/",
];

/// Allowed temp paths for writing
/// All other /tmp writes are blocked
const ALLOWED_TMP_PREFIXES: &[&str] = &[
    "/tmp/sage/",
    "/tmp/sage-agent/",
    "/private/tmp/sage/",      // macOS
    "/private/tmp/sage-agent/", // macOS
];

/// Path access policy
#[derive(Debug)]
pub struct PathPolicy {
    /// Allowed read paths (canonicalized)
    allowed_read_paths: Vec<PathBuf>,

    /// Allowed write paths (canonicalized)
    allowed_write_paths: Vec<PathBuf>,

    /// Denied paths (always blocked)
    denied_paths: Vec<PathBuf>,

    /// Additional sensitive file patterns
    sensitive_patterns: Vec<String>,

    /// Additional allowed tmp paths
    allowed_tmp_paths: Vec<PathBuf>,

    /// Working directory
    working_dir: Option<PathBuf>,

    /// Allow all reads (permissive mode)
    allow_all_reads: bool,

    /// Strict mode for sensitive files
    strict_sensitive_files: bool,
}

impl PathPolicy {
    /// Create path policy from configuration
    pub fn from_config(config: &SandboxConfig) -> Result<Self, SandboxError> {
        let mut allowed_read_paths = Vec::new();
        let mut allow_all_reads = false;

        for path in &config.allowed_read_paths {
            if path.as_os_str() == "/" {
                allow_all_reads = true;
            } else {
                allowed_read_paths.push(path.clone());
            }
        }

        // Add working directory to both read and write paths
        if let Some(ref work_dir) = config.working_dir {
            allowed_read_paths.push(work_dir.clone());
        }

        let mut allowed_write_paths = config.allowed_write_paths.clone();
        if let Some(ref work_dir) = config.working_dir {
            allowed_write_paths.push(work_dir.clone());
        }

        Ok(Self {
            allowed_read_paths,
            allowed_write_paths,
            denied_paths: Self::default_denied_paths(),
            sensitive_patterns: Vec::new(),
            allowed_tmp_paths: Vec::new(),
            working_dir: config.working_dir.clone(),
            allow_all_reads,
            strict_sensitive_files: true,
        })
    }

    /// Add additional sensitive file patterns
    pub fn with_sensitive_patterns(mut self, patterns: Vec<String>) -> Self {
        self.sensitive_patterns = patterns;
        self
    }

    /// Add additional allowed tmp paths
    pub fn with_allowed_tmp_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_tmp_paths = paths;
        self
    }

    /// Set strict mode for sensitive files
    pub fn with_strict_sensitive_files(mut self, strict: bool) -> Self {
        self.strict_sensitive_files = strict;
        self
    }

    /// Check if a path is a sensitive file
    pub fn is_sensitive_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check built-in sensitive files
        for pattern in SENSITIVE_FILES {
            if pattern.ends_with('/') {
                // Directory pattern
                let dir_pattern = pattern.trim_end_matches('/');
                if path_str.contains(&format!("/{}/", dir_pattern))
                    || path_str.ends_with(&format!("/{}", dir_pattern))
                {
                    return true;
                }
            } else {
                // File pattern
                if path_str.ends_with(pattern) || path_str.contains(&format!("/{}", pattern)) {
                    return true;
                }
            }
        }

        // Check additional patterns
        for pattern in &self.sensitive_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Check if a path is in an allowed tmp location
    pub fn is_allowed_tmp_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check if it's a tmp path at all
        if !path_str.starts_with("/tmp") && !path_str.starts_with("/private/tmp") {
            return true; // Not a tmp path, so not restricted by tmp rules
        }

        // Check built-in allowed prefixes
        for prefix in ALLOWED_TMP_PREFIXES {
            if path_str.starts_with(prefix) {
                return true;
            }
        }

        // Check additional allowed paths
        for allowed in &self.allowed_tmp_paths {
            let allowed_str = allowed.to_string_lossy();
            if path_str.starts_with(allowed_str.as_ref()) {
                return true;
            }
        }

        false
    }

    /// Enhanced path check with sensitive file and tmp restrictions
    pub fn check_path_enhanced(&self, path: &Path, write: bool) -> Result<(), SandboxError> {
        // First do the standard check
        self.check_path(&path.to_path_buf(), write)?;

        // Additional checks for write operations
        if write {
            // Check sensitive files
            if self.strict_sensitive_files && self.is_sensitive_file(path) {
                return Err(SandboxError::PathAccessDenied {
                    path: format!("{} (sensitive file)", path.display()),
                });
            }

            // Check tmp path restrictions
            if !self.is_allowed_tmp_path(path) {
                return Err(SandboxError::PathAccessDenied {
                    path: format!("{} (use /tmp/sage/ for temp files)", path.display()),
                });
            }
        }

        Ok(())
    }

    /// Default denied paths (system-critical)
    fn default_denied_paths() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/etc/passwd"),
            PathBuf::from("/etc/shadow"),
            PathBuf::from("/etc/sudoers"),
            PathBuf::from("/root"),
            PathBuf::from("/var/log"),
            PathBuf::from("/proc"),
            PathBuf::from("/sys"),
            PathBuf::from("/dev"),
        ]
    }

    /// Check if path access is allowed
    pub fn check_path(&self, path: &PathBuf, write: bool) -> Result<(), SandboxError> {
        // Resolve the path (try to normalize it)
        let resolved = self.normalize_path(path);

        // Check denied paths first
        for denied in &self.denied_paths {
            let denied_normalized = self.normalize_path(denied);
            if resolved.starts_with(&denied_normalized) {
                return Err(SandboxError::PathAccessDenied {
                    path: path.display().to_string(),
                });
            }
        }

        if write {
            // Check write access
            let allowed = self.allowed_write_paths.iter().any(|allowed| {
                let allowed_normalized = self.normalize_path(allowed);
                resolved.starts_with(&allowed_normalized)
            });

            if !allowed {
                return Err(SandboxError::PathAccessDenied {
                    path: format!("{} (write)", path.display()),
                });
            }
        } else {
            // Check read access
            if !self.allow_all_reads {
                let allowed = self.allowed_read_paths.iter().any(|allowed| {
                    let allowed_normalized = self.normalize_path(allowed);
                    resolved.starts_with(&allowed_normalized)
                });

                if !allowed {
                    return Err(SandboxError::PathAccessDenied {
                        path: format!("{} (read)", path.display()),
                    });
                }
            }
        }

        Ok(())
    }

    /// Normalize a path by resolving symlinks and making it absolute
    /// Falls back to just making it absolute if canonicalization fails
    fn normalize_path(&self, path: &PathBuf) -> PathBuf {
        // First try to canonicalize (resolves symlinks)
        if let Ok(canonical) = path.canonicalize() {
            return canonical;
        }

        // If the path doesn't exist, try to normalize what we can
        let absolute = if path.is_absolute() {
            path.clone()
        } else if let Some(ref work_dir) = self.working_dir {
            work_dir.join(path)
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.clone())
        };

        // Try to canonicalize the parent if it exists
        if let Some(parent) = absolute.parent() {
            if let Ok(canonical_parent) = parent.canonicalize() {
                if let Some(file_name) = absolute.file_name() {
                    return canonical_parent.join(file_name);
                }
            }
        }

        absolute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_temp_dir() -> PathBuf {
        // On macOS, /tmp is symlinked to /private/tmp
        std::env::temp_dir()
    }

    #[test]
    fn test_path_policy_read() {
        let temp_dir = get_temp_dir();
        let config = SandboxConfig {
            allowed_read_paths: vec![temp_dir.clone()],
            ..Default::default()
        };
        let policy = PathPolicy::from_config(&config).unwrap();

        // Allowed read
        let test_path = temp_dir.join("test.txt");
        assert!(policy.check_path(&test_path, false).is_ok());

        // Denied read (not in allowed paths)
        assert!(
            policy
                .check_path(&PathBuf::from("/var/test.txt"), false)
                .is_err()
        );
    }

    #[test]
    fn test_path_policy_write() {
        let temp_dir = get_temp_dir();
        let config = SandboxConfig {
            allowed_write_paths: vec![temp_dir.clone()],
            ..Default::default()
        };
        let policy = PathPolicy::from_config(&config).unwrap();

        // Allowed write
        let test_path = temp_dir.join("test.txt");
        assert!(policy.check_path(&test_path, true).is_ok());

        // Denied write
        assert!(
            policy
                .check_path(&PathBuf::from("/var/test.txt"), true)
                .is_err()
        );
    }

    #[test]
    fn test_path_policy_denied() {
        let config = SandboxConfig {
            allowed_read_paths: vec![PathBuf::from("/")],
            ..Default::default()
        };
        let policy = PathPolicy::from_config(&config).unwrap();

        // System paths are always denied
        assert!(
            policy
                .check_path(&PathBuf::from("/etc/shadow"), false)
                .is_err()
        );
        assert!(
            policy
                .check_path(&PathBuf::from("/proc/1/status"), false)
                .is_err()
        );
    }

    #[test]
    fn test_sensitive_file_detection() {
        let config = SandboxConfig::default();
        let policy = PathPolicy::from_config(&config).unwrap();

        // Git config
        assert!(policy.is_sensitive_file(Path::new("/home/user/.gitconfig")));
        assert!(policy.is_sensitive_file(Path::new("/Users/user/.git/config")));
        assert!(policy.is_sensitive_file(Path::new("/project/.git/hooks/pre-commit")));

        // Shell config
        assert!(policy.is_sensitive_file(Path::new("/home/user/.bashrc")));
        assert!(policy.is_sensitive_file(Path::new("/home/user/.zshrc")));

        // SSH
        assert!(policy.is_sensitive_file(Path::new("/home/user/.ssh/id_rsa")));
        assert!(policy.is_sensitive_file(Path::new("/home/user/.ssh/config")));

        // AWS/Docker/Kube
        assert!(policy.is_sensitive_file(Path::new("/home/user/.aws/credentials")));
        assert!(policy.is_sensitive_file(Path::new("/home/user/.docker/config.json")));
        assert!(policy.is_sensitive_file(Path::new("/home/user/.kube/config")));

        // Env files
        assert!(policy.is_sensitive_file(Path::new("/project/.env")));
        assert!(policy.is_sensitive_file(Path::new("/project/.env.local")));

        // Non-sensitive files
        assert!(!policy.is_sensitive_file(Path::new("/project/src/main.rs")));
        assert!(!policy.is_sensitive_file(Path::new("/project/Cargo.toml")));
    }

    #[test]
    fn test_allowed_tmp_path() {
        let config = SandboxConfig::default();
        let policy = PathPolicy::from_config(&config).unwrap();

        // Allowed sage tmp paths
        assert!(policy.is_allowed_tmp_path(Path::new("/tmp/sage/test.txt")));
        assert!(policy.is_allowed_tmp_path(Path::new("/tmp/sage-agent/output")));
        assert!(policy.is_allowed_tmp_path(Path::new("/private/tmp/sage/test")));

        // Disallowed tmp paths
        assert!(!policy.is_allowed_tmp_path(Path::new("/tmp/random/file")));
        assert!(!policy.is_allowed_tmp_path(Path::new("/tmp/test.txt")));

        // Non-tmp paths are allowed (not restricted by tmp rules)
        assert!(policy.is_allowed_tmp_path(Path::new("/home/user/project/file")));
    }

    #[test]
    fn test_custom_sensitive_patterns() {
        let config = SandboxConfig::default();
        let policy = PathPolicy::from_config(&config)
            .unwrap()
            .with_sensitive_patterns(vec!["custom_secret".to_string()]);

        assert!(policy.is_sensitive_file(Path::new("/project/custom_secret.txt")));
        assert!(policy.is_sensitive_file(Path::new("/project/config/custom_secret")));
    }

    #[test]
    fn test_custom_allowed_tmp_paths() {
        let config = SandboxConfig::default();
        let policy = PathPolicy::from_config(&config)
            .unwrap()
            .with_allowed_tmp_paths(vec![PathBuf::from("/tmp/myapp/")]);

        assert!(policy.is_allowed_tmp_path(Path::new("/tmp/myapp/test.txt")));
    }
}
