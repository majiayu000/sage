//! Dangerous removal check following Claude Code patterns.
//!
//! Detects potentially dangerous file/directory removal:
//! - rm -rf / or equivalent
//! - Removal of critical system paths
//! - Recursive removal of home directories

use super::types::{CheckType, ValidationResult, ValidationWarning};
use regex::Regex;
use std::sync::LazyLock;

/// Critical paths that should never be removed
const CRITICAL_PATHS: &[&str] = &[
    "/",
    "/bin",
    "/boot",
    "/dev",
    "/etc",
    "/home",
    "/lib",
    "/lib64",
    "/opt",
    "/proc",
    "/root",
    "/run",
    "/sbin",
    "/srv",
    "/sys",
    "/tmp",
    "/usr",
    "/var",
    // macOS specific
    "/Applications",
    "/Library",
    "/System",
    "/Users",
    "/Volumes",
    "/cores",
    "/private",
    // User directories
    "~",
    "$HOME",
    "${HOME}",
];

/// Pattern for rm command with recursive flag
static RM_RECURSIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\brm\s+(?:-[a-zA-Z]*[rR][a-zA-Z]*\s+|--recursive\s+)"#).unwrap()
});

/// Pattern for rm with force and recursive
static RM_RF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\brm\s+-[a-zA-Z]*[rR][a-zA-Z]*[fF][a-zA-Z]*\s+|\brm\s+-[a-zA-Z]*[fF][a-zA-Z]*[rR][a-zA-Z]*\s+"#).unwrap()
});

/// Pattern to extract rm target path
static RM_TARGET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\brm\s+(?:-[a-zA-Z]+\s+)*([^\s|;&]+)"#).unwrap()
});

/// Pattern for wildcard in path
static WILDCARD_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(/[^/\s]*\*|/\.\.\s|/\.\.$)"#).unwrap()
});

/// Check for dangerous file removal commands
///
/// Blocks commands that attempt to remove:
/// - Root filesystem (rm -rf /)
/// - Critical system paths (/bin, /etc, /usr, etc.)
/// - Home directories with recursive flag
///
/// Warns about:
/// - Any recursive removal
/// - Wildcard in removal path
pub fn check_dangerous_removal(command: &str) -> ValidationResult {
    let mut warnings = Vec::new();

    // Find rm command target
    if let Some(caps) = RM_TARGET.captures(command) {
        let target = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let target_normalized = normalize_path(target);

        // Check if targeting critical path
        for critical in CRITICAL_PATHS {
            if is_path_match(&target_normalized, critical) {
                return ValidationResult::block(
                    CheckType::DangerousRemoval,
                    format!(
                        "Removal of critical path '{}' is blocked for safety",
                        target
                    ),
                );
            }
        }

        // Check for rm -rf with wildcards at root level
        if RM_RF.is_match(command) && WILDCARD_PATH.is_match(target) {
            return ValidationResult::block(
                CheckType::DangerousRemoval,
                format!(
                    "Recursive removal with wildcard '{}' is too dangerous",
                    target
                ),
            );
        }

        // Check for .. traversal that could escape to critical paths
        if target.contains("..") && RM_RECURSIVE.is_match(command) {
            warnings.push(ValidationWarning::critical(
                format!(
                    "Recursive removal with '..' traversal '{}' - verify target carefully",
                    target
                ),
            ));
        }

        // Warn about any recursive removal
        if RM_RECURSIVE.is_match(command) || RM_RF.is_match(command) {
            if warnings.is_empty() {
                warnings.push(ValidationWarning::warning(format!(
                    "Recursive removal of '{}' - ensure this is intentional",
                    target
                )));
            }
        }
    }

    if warnings.is_empty() {
        ValidationResult::pass(CheckType::DangerousRemoval)
    } else {
        ValidationResult::pass_with_warnings(CheckType::DangerousRemoval, warnings)
    }
}

/// Normalize a path for comparison
fn normalize_path(path: &str) -> String {
    let mut normalized = path.to_string();

    // Expand ~ to $HOME for comparison
    if normalized.starts_with('~') {
        normalized = normalized.replacen('~', "$HOME", 1);
    }

    // Remove trailing slashes for comparison
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    normalized
}

/// Check if a path matches a critical path pattern
fn is_path_match(path: &str, critical: &str) -> bool {
    // Exact match
    if path == critical {
        return true;
    }

    // Path starts with critical path as a directory
    if path.starts_with(critical) {
        let after = &path[critical.len()..];
        if after.is_empty() || after.starts_with('/') {
            // Only match if it's the exact path or a subdirectory
            // But for root paths, we need to be more careful
            if critical == "/" {
                return true;
            }
        }
    }

    // Handle $HOME and ~ equivalence
    if critical.contains("HOME") || critical == "~" {
        if path.starts_with("$HOME") || path.starts_with("${HOME}") || path.starts_with('~') {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::validation::WarningSeverity;

    #[test]
    fn test_safe_rm() {
        let result = check_dangerous_removal("rm file.txt");
        assert!(result.allowed);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_rm_rf_root() {
        let result = check_dangerous_removal("rm -rf /");
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("critical path"));
    }

    #[test]
    fn test_rm_rf_etc() {
        let result = check_dangerous_removal("rm -rf /etc");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_rf_usr() {
        let result = check_dangerous_removal("rm -rf /usr/");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_rf_home() {
        let result = check_dangerous_removal("rm -rf /home");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_rf_tilde() {
        let result = check_dangerous_removal("rm -rf ~");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_home_var() {
        let result = check_dangerous_removal("rm -rf $HOME");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_safe_subdir() {
        let result = check_dangerous_removal("rm -rf /tmp/mydir/subdir");
        assert!(result.allowed);
        // Should still warn about recursive removal
        assert!(result.warnings.iter().any(|w| w.message.contains("Recursive")));
    }

    #[test]
    fn test_rm_wildcard_warning() {
        let result = check_dangerous_removal("rm -rf /tmp/*.log");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_dot_dot_warning() {
        let result = check_dangerous_removal("rm -r ../important");
        assert!(result.allowed);
        assert!(result.warnings.iter().any(|w| w.severity == WarningSeverity::Critical));
    }

    #[test]
    fn test_rm_macos_applications() {
        let result = check_dangerous_removal("rm -rf /Applications");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_users() {
        let result = check_dangerous_removal("rm -rf /Users");
        assert!(!result.allowed);
    }

    #[test]
    fn test_non_rm_command() {
        let result = check_dangerous_removal("ls -la /");
        assert!(result.allowed);
        assert!(result.warnings.is_empty());
    }
}
