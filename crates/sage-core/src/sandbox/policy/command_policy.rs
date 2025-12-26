//! Command execution policy

use super::super::config::SandboxConfig;
use super::super::SandboxError;
use regex::Regex;
use std::collections::HashSet;

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
        let allowed_commands: HashSet<String> = config.allowed_commands.iter().cloned().collect();
        let blocked_commands: HashSet<String> = config.blocked_commands.iter().cloned().collect();

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
