//! Path access policy

use super::super::config::SandboxConfig;
use super::super::SandboxError;
use std::path::PathBuf;

/// Path access policy
#[derive(Debug)]
pub struct PathPolicy {
    /// Allowed read paths (canonicalized)
    allowed_read_paths: Vec<PathBuf>,

    /// Allowed write paths (canonicalized)
    allowed_write_paths: Vec<PathBuf>,

    /// Denied paths (always blocked)
    denied_paths: Vec<PathBuf>,

    /// Working directory
    working_dir: Option<PathBuf>,

    /// Allow all reads (permissive mode)
    allow_all_reads: bool,
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
            working_dir: config.working_dir.clone(),
            allow_all_reads,
        })
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
}
