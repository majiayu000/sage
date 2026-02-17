//! Command execution policy

use super::super::SandboxError;
use super::super::config::SandboxConfig;
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

        // Check for shell-with-command pattern (e.g., "bash -c 'rm -rf /'")
        if let Some(inner_cmd) = self.extract_shell_command(command) {
            let inner_base = self.extract_base_command(&inner_cmd);
            if self.blocked_commands.contains(&inner_base) {
                return Err(SandboxError::CommandNotAllowed {
                    command: format!("{} (inner command blocked: {})", command, inner_base),
                });
            }
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

    /// Extract inner command from shell-with-command pattern
    /// Detects patterns like: bash -c "command", sh -c 'command', etc.
    fn extract_shell_command(&self, command: &str) -> Option<String> {
        let shells = ["bash", "sh", "zsh", "csh", "fish", "dash", "ksh", "tcsh"];

        let tokens: Vec<&str> = command.split_whitespace().collect();
        if tokens.len() < 3 {
            return None;
        }

        let base = self.extract_base_command(command);
        if !shells.contains(&base.as_str()) {
            return None;
        }

        // Check if second token is -c
        if tokens[1] != "-c" {
            return None;
        }

        // Extract the command string (may be quoted or unquoted)
        // Join remaining tokens and strip quotes
        let cmd_str = tokens[2..].join(" ");
        let trimmed = cmd_str.trim();

        // Remove surrounding quotes if present
        let inner = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };

        if inner.is_empty() {
            None
        } else {
            Some(inner.to_string())
        }
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

    #[test]
    fn test_shell_with_command_bypass() {
        let config = SandboxConfig::default();
        let policy = CommandPolicy::from_config(&config).unwrap();

        // Shell-with-command patterns should be blocked
        assert!(policy.check_command("bash -c 'rm -rf /'").is_err());
        assert!(policy.check_command("sh -c 'sudo ls'").is_err());
        assert!(policy.check_command("zsh -c 'kill -9 1'").is_err());
        assert!(policy.check_command("bash -c \"rm -rf /\"").is_err());
        assert!(policy.check_command("fish -c 'chmod 777 /etc'").is_err());

        // Safe shell commands should still work if shell is allowed
        // (but in default config, shells are blocked anyway)
        assert!(policy.check_command("bash -c 'echo hello'").is_err()); // bash itself blocked
    }

    #[test]
    fn test_extract_shell_command() {
        let config = SandboxConfig::default();
        let policy = CommandPolicy::from_config(&config).unwrap();

        // Test extraction
        assert_eq!(
            policy.extract_shell_command("bash -c 'rm -rf /'"),
            Some("rm -rf /".to_string())
        );
        assert_eq!(
            policy.extract_shell_command("sh -c \"sudo ls\""),
            Some("sudo ls".to_string())
        );
        assert_eq!(
            policy.extract_shell_command("zsh -c kill"),
            Some("kill".to_string())
        );

        // Non-shell commands
        assert_eq!(policy.extract_shell_command("ls -la"), None);
        assert_eq!(policy.extract_shell_command("git status"), None);

        // Shell without -c
        assert_eq!(policy.extract_shell_command("bash script.sh"), None);
    }
}
