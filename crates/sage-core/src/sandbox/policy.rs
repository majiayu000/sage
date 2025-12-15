//! Sandbox policies for access control

use super::config::SandboxConfig;
use super::SandboxError;
use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;

/// Combined sandbox policy
#[derive(Debug)]
pub struct SandboxPolicy {
    pub path_policy: PathPolicy,
    pub command_policy: CommandPolicy,
    pub network_policy: NetworkPolicy,
}

impl SandboxPolicy {
    /// Create policy from configuration
    pub fn from_config(config: &SandboxConfig) -> Result<Self, SandboxError> {
        Ok(Self {
            path_policy: PathPolicy::from_config(config)?,
            command_policy: CommandPolicy::from_config(config)?,
            network_policy: NetworkPolicy::from_config(config)?,
        })
    }
}

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

/// Command execution policy
#[derive(Debug)]
pub struct CommandPolicy {
    /// Allowed commands (empty means all allowed)
    allowed_commands: HashSet<String>,

    /// Blocked commands (always checked)
    blocked_commands: HashSet<String>,

    /// Blocked command patterns (regex)
    blocked_patterns: Vec<Regex>,

    /// Allow all commands (permissive mode)
    allow_all: bool,
}

impl CommandPolicy {
    /// Create command policy from configuration
    pub fn from_config(config: &SandboxConfig) -> Result<Self, SandboxError> {
        let allowed_commands: HashSet<String> =
            config.allowed_commands.iter().cloned().collect();
        let blocked_commands: HashSet<String> =
            config.blocked_commands.iter().cloned().collect();

        let allow_all = allowed_commands.is_empty();

        // Build blocked patterns
        let blocked_patterns = vec![
            // Shell metacharacters abuse
            Regex::new(r";\s*(rm|sudo|dd|mkfs)").ok(),
            // Command substitution
            Regex::new(r"\$\(").ok(),
            Regex::new(r"`").ok(),
            // Pipe to dangerous commands
            Regex::new(r"\|\s*(sh|bash|zsh|rm|sudo)").ok(),
            // Redirect to sensitive files
            Regex::new(r">\s*/etc/").ok(),
            Regex::new(r">\s*/dev/").ok(),
        ]
        .into_iter()
        .flatten()
        .collect();

        Ok(Self {
            allowed_commands,
            blocked_commands,
            blocked_patterns,
            allow_all,
        })
    }

    /// Check if command is allowed
    pub fn check_command(&self, command: &str) -> Result<(), SandboxError> {
        // Extract base command
        let base_command = self.extract_base_command(command);

        // Check blocked commands
        if self.blocked_commands.contains(&base_command) {
            return Err(SandboxError::CommandNotAllowed {
                command: command.to_string(),
            });
        }

        // Check blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(command) {
                return Err(SandboxError::CommandNotAllowed {
                    command: format!("{} (matches blocked pattern)", command),
                });
            }
        }

        // Check allowed commands
        if !self.allow_all && !self.allowed_commands.contains(&base_command) {
            return Err(SandboxError::CommandNotAllowed {
                command: command.to_string(),
            });
        }

        Ok(())
    }

    /// Extract base command from command string
    fn extract_base_command(&self, command: &str) -> String {
        command
            .split_whitespace()
            .next()
            .map(|s| {
                std::path::Path::new(s)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(s)
                    .to_string()
            })
            .unwrap_or_default()
    }
}

/// Network access policy
#[derive(Debug)]
pub struct NetworkPolicy {
    /// Whether network is allowed
    enabled: bool,

    /// Allowed hosts
    allowed_hosts: HashSet<String>,

    /// Blocked hosts
    blocked_hosts: HashSet<String>,

    /// Allowed ports
    allowed_ports: HashSet<u16>,

    /// Blocked ports
    blocked_ports: HashSet<u16>,

    /// Allow all hosts (if allowed_hosts is empty)
    allow_all_hosts: bool,
}

impl NetworkPolicy {
    /// Create network policy from configuration
    pub fn from_config(config: &SandboxConfig) -> Result<Self, SandboxError> {
        let allowed_hosts: HashSet<String> = config.allowed_hosts.iter().cloned().collect();
        let blocked_hosts: HashSet<String> = config.blocked_hosts.iter().cloned().collect();
        let allow_all_hosts = allowed_hosts.is_empty();

        Ok(Self {
            enabled: config.allow_network,
            allowed_hosts,
            blocked_hosts,
            allowed_ports: HashSet::new(), // All ports allowed by default
            blocked_ports: Self::default_blocked_ports(),
            allow_all_hosts,
        })
    }

    /// Default blocked ports (dangerous services)
    fn default_blocked_ports() -> HashSet<u16> {
        vec![
            22,   // SSH
            23,   // Telnet
            25,   // SMTP
            110,  // POP3
            143,  // IMAP
            445,  // SMB
            3306, // MySQL
            5432, // PostgreSQL
            6379, // Redis
            27017, // MongoDB
        ]
        .into_iter()
        .collect()
    }

    /// Check if network access is allowed
    pub fn check_access(&self, host: &str, port: u16) -> Result<(), SandboxError> {
        // Check if network is enabled
        if !self.enabled {
            return Err(SandboxError::NetworkAccessDenied {
                host: format!("{}:{} (network disabled)", host, port),
            });
        }

        // Check blocked hosts
        if self.blocked_hosts.iter().any(|h| host.contains(h)) {
            return Err(SandboxError::NetworkAccessDenied {
                host: format!("{}:{} (blocked host)", host, port),
            });
        }

        // Check allowed hosts
        if !self.allow_all_hosts && !self.allowed_hosts.iter().any(|h| host.contains(h)) {
            return Err(SandboxError::NetworkAccessDenied {
                host: format!("{}:{} (not in allowed list)", host, port),
            });
        }

        // Check blocked ports
        if self.blocked_ports.contains(&port) {
            return Err(SandboxError::NetworkAccessDenied {
                host: format!("{}:{} (blocked port)", host, port),
            });
        }

        // Check allowed ports (if specified)
        if !self.allowed_ports.is_empty() && !self.allowed_ports.contains(&port) {
            return Err(SandboxError::NetworkAccessDenied {
                host: format!("{}:{} (port not allowed)", host, port),
            });
        }

        Ok(())
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
        assert!(policy.check_path(&PathBuf::from("/var/test.txt"), false).is_err());
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
        assert!(policy.check_path(&PathBuf::from("/var/test.txt"), true).is_err());
    }

    #[test]
    fn test_path_policy_denied() {
        let config = SandboxConfig {
            allowed_read_paths: vec![PathBuf::from("/")],
            ..Default::default()
        };
        let policy = PathPolicy::from_config(&config).unwrap();

        // System paths are always denied
        assert!(policy.check_path(&PathBuf::from("/etc/shadow"), false).is_err());
        assert!(policy.check_path(&PathBuf::from("/proc/1/status"), false).is_err());
    }

    #[test]
    fn test_command_policy_allowed() {
        let config = SandboxConfig::default();
        let policy = CommandPolicy::from_config(&config).unwrap();

        // Allowed commands
        assert!(policy.check_command("ls -la").is_ok());
        assert!(policy.check_command("git status").is_ok());
    }

    #[test]
    fn test_command_policy_blocked() {
        let config = SandboxConfig::default();
        let policy = CommandPolicy::from_config(&config).unwrap();

        // Blocked commands
        assert!(policy.check_command("rm -rf /").is_err());
        assert!(policy.check_command("sudo ls").is_err());
    }

    #[test]
    fn test_command_policy_patterns() {
        let config = SandboxConfig::default();
        let policy = CommandPolicy::from_config(&config).unwrap();

        // Blocked patterns
        assert!(policy.check_command("echo $(whoami)").is_err());
        assert!(policy.check_command("cat /etc/passwd | sh").is_err());
    }

    #[test]
    fn test_network_policy_enabled() {
        let config = SandboxConfig {
            allow_network: true,
            blocked_hosts: vec!["blocked.com".to_string()],
            ..Default::default()
        };
        let policy = NetworkPolicy::from_config(&config).unwrap();

        // Allowed access
        assert!(policy.check_access("example.com", 443).is_ok());

        // Blocked host
        assert!(policy.check_access("blocked.com", 443).is_err());

        // Blocked port
        assert!(policy.check_access("example.com", 22).is_err());
    }

    #[test]
    fn test_network_policy_disabled() {
        let config = SandboxConfig {
            allow_network: false,
            ..Default::default()
        };
        let policy = NetworkPolicy::from_config(&config).unwrap();

        // All network access denied
        assert!(policy.check_access("example.com", 443).is_err());
    }
}
