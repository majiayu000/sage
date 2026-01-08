//! Network access policy

use super::super::SandboxError;
use super::super::config::SandboxConfig;
use std::collections::HashSet;

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
            22,    // SSH
            23,    // Telnet
            25,    // SMTP
            110,   // POP3
            143,   // IMAP
            445,   // SMB
            3306,  // MySQL
            5432,  // PostgreSQL
            6379,  // Redis
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
