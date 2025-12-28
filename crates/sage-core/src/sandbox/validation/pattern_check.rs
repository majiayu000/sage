//! Dangerous pattern detection following Claude Code patterns.
//!
//! Detects potentially dangerous command patterns:
//! - Backtick command substitution
//! - eval usage
//! - Network access commands
//! - Process manipulation

use super::types::{CheckType, ValidationResult, ValidationWarning, WarningSeverity};
use regex::Regex;
use std::sync::LazyLock;

/// Pattern for backtick command substitution
static BACKTICK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"`[^`]+`"#).unwrap()
});

/// Pattern for eval command
static EVAL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\beval\s+"#).unwrap()
});

/// Pattern for network commands
static NETWORK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(curl|wget|nc|netcat|ncat|ssh|scp|rsync|ftp|sftp)\b"#).unwrap()
});

/// Pattern for process manipulation
static PROCESS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(kill|pkill|killall|nohup|disown)\b"#).unwrap()
});

/// Pattern for shell spawning
static SHELL_SPAWN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(bash|sh|zsh|fish|csh|tcsh|ksh)\s+-c\s+"#).unwrap()
});

/// Pattern for sudo/su/pkexec
static PRIVILEGE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(sudo|su|doas|pkexec)\b"#).unwrap()
});

/// Pattern for filesystem destruction commands
static FS_DESTROY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(mkfs|mkfs\.\w+|wipefs|shred)\b"#).unwrap()
});

/// Pattern for base64 decoding (potential obfuscation)
static OBFUSCATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(base64\s+-d|base64\s+--decode)\b"#).unwrap()
});

/// Check for dangerous command patterns
///
/// Blocks commands with:
/// - eval usage
/// - sudo/su (privilege escalation)
///
/// Warns about:
/// - Backtick command substitution
/// - Network access
/// - Process manipulation
/// - Shell spawning with -c
/// - Base64 decoding
pub fn check_dangerous_patterns(command: &str) -> ValidationResult {
    let mut warnings = Vec::new();

    // Block: eval command
    if EVAL_PATTERN.is_match(command) {
        return ValidationResult::block(
            CheckType::DangerousPattern,
            "eval command is not allowed - potential code injection",
        );
    }

    // Block: privilege escalation
    if PRIVILEGE_PATTERN.is_match(command) {
        return ValidationResult::block(
            CheckType::DangerousPattern,
            "sudo/su/pkexec commands are not allowed - use sandboxed execution",
        );
    }

    // Block: filesystem destruction
    if FS_DESTROY_PATTERN.is_match(command) {
        return ValidationResult::block(
            CheckType::DangerousPattern,
            "Filesystem destruction commands (mkfs, wipefs, shred) are not allowed",
        );
    }

    // Warn: backtick substitution
    if BACKTICK_PATTERN.is_match(command) {
        warnings.push(ValidationWarning::with_suggestion(
            "Command uses backtick substitution",
            WarningSeverity::Warning,
            "Prefer $() syntax for command substitution",
        ));
    }

    // Warn: network access
    if let Some(captures) = NETWORK_PATTERN.captures(command) {
        let cmd = captures.get(1).map(|m| m.as_str()).unwrap_or("network");
        warnings.push(ValidationWarning::warning(format!(
            "Command uses network tool '{}' - ensure URL/host is trusted",
            cmd
        )));
    }

    // Warn: process manipulation
    if let Some(captures) = PROCESS_PATTERN.captures(command) {
        let cmd = captures.get(1).map(|m| m.as_str()).unwrap_or("process");
        warnings.push(ValidationWarning::warning(format!(
            "Command manipulates processes with '{}' - ensure target is correct",
            cmd
        )));
    }

    // Warn: shell spawning
    if SHELL_SPAWN_PATTERN.is_match(command) {
        warnings.push(ValidationWarning::warning(
            "Command spawns a new shell with -c - command string will be executed",
        ));
    }

    // Warn: potential obfuscation
    if OBFUSCATION_PATTERN.is_match(command) {
        warnings.push(ValidationWarning::critical(
            "Command decodes base64 - potential obfuscated payload",
        ));
    }

    if warnings.is_empty() {
        ValidationResult::pass(CheckType::DangerousPattern)
    } else {
        ValidationResult::pass_with_warnings(CheckType::DangerousPattern, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_command() {
        let result = check_dangerous_patterns("ls -la");
        assert!(result.allowed);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_eval_blocked() {
        let result = check_dangerous_patterns("eval $COMMAND");
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("eval"));
    }

    #[test]
    fn test_sudo_blocked() {
        let result = check_dangerous_patterns("sudo apt install package");
        assert!(!result.allowed);
    }

    #[test]
    fn test_su_blocked() {
        let result = check_dangerous_patterns("su - root");
        assert!(!result.allowed);
    }

    #[test]
    fn test_backtick_warning() {
        let result = check_dangerous_patterns("echo `date`");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.message.contains("backtick")));
    }

    #[test]
    fn test_curl_warning() {
        let result = check_dangerous_patterns("curl https://example.com");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.message.contains("curl")));
    }

    #[test]
    fn test_wget_warning() {
        let result = check_dangerous_patterns("wget http://example.com/file");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.message.contains("wget")));
    }

    #[test]
    fn test_kill_warning() {
        let result = check_dangerous_patterns("kill -9 1234");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.message.contains("kill")));
    }

    #[test]
    fn test_bash_c_warning() {
        let result = check_dangerous_patterns("bash -c 'echo hello'");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.message.contains("shell")));
    }

    #[test]
    fn test_base64_decode_warning() {
        let result = check_dangerous_patterns("echo SGVsbG8= | base64 -d");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.severity == WarningSeverity::Critical));
    }

    #[test]
    fn test_ssh_warning() {
        let result = check_dangerous_patterns("ssh user@host");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.message.contains("ssh")));
    }
}
