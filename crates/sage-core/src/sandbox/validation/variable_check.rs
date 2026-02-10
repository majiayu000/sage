//! Dangerous variable usage check following Claude Code patterns.
//!
//! Detects potentially dangerous variable usage in commands:
//! - Variables in redirect targets: > $file
//! - Variables in critical positions
//! - Unquoted variable expansion

use super::types::{CheckType, CommandValidationResult, ValidationWarning, WarningSeverity};
use regex::Regex;
use std::sync::LazyLock;

/// Pattern to detect variables in redirect targets
static REDIRECT_VARIABLE: LazyLock<Regex> = LazyLock::new(|| {
    // Match > $var, >> $var, 2> $var, etc.
    Regex::new(r#"[0-9]*>+\s*\$\{?[a-zA-Z_][a-zA-Z0-9_]*\}?"#).unwrap()
});

/// Pattern to detect unquoted variable expansion
static UNQUOTED_VARIABLE: LazyLock<Regex> = LazyLock::new(|| {
    // Match $var not inside quotes (simplified check)
    Regex::new(r#"(?:^|[^'"\\])\$\{?[a-zA-Z_][a-zA-Z0-9_]*\}?"#).unwrap()
});

/// Pattern for variable in rm target
static RM_VARIABLE: LazyLock<Regex> = LazyLock::new(|| {
    // Match rm with variable argument
    Regex::new(r#"\brm\s+(?:-[rfivI]+\s+)*\$\{?[a-zA-Z_][a-zA-Z0-9_]*\}?"#).unwrap()
});

/// Pattern for variable in chmod/chown target
static PERM_VARIABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:chmod|chown)\s+\S+\s+\$\{?[a-zA-Z_][a-zA-Z0-9_]*\}?"#).unwrap()
});

/// Check for dangerous variable usage
///
/// Blocks commands with:
/// - Variables in redirect targets (> $file)
/// - Variables as rm targets
///
/// Warns about:
/// - Unquoted variable expansion
/// - Variables in permission commands
pub fn check_dangerous_variables(command: &str) -> CommandValidationResult {
    let mut warnings = Vec::new();

    // Block: Variable in redirect target
    if REDIRECT_VARIABLE.is_match(command) {
        return CommandValidationResult::block_with_warnings(
            CheckType::DangerousVariable,
            "Variable in redirect target is dangerous - could overwrite arbitrary files",
            vec![ValidationWarning::with_suggestion(
                "Redirect target contains variable expansion",
                WarningSeverity::Critical,
                "Use explicit file path instead of variable",
            )],
        );
    }

    // Block: Variable as rm target (high risk)
    if RM_VARIABLE.is_match(command) {
        return CommandValidationResult::block(
            CheckType::DangerousVariable,
            "Variable as rm target is dangerous - could delete arbitrary files",
        );
    }

    // Warn: Variable in chmod/chown
    if PERM_VARIABLE.is_match(command) {
        warnings.push(ValidationWarning::warning(
            "Variable in permission command - ensure the path is validated",
        ));
    }

    // Warn: Unquoted variable (lower priority)
    if UNQUOTED_VARIABLE.is_match(command) {
        // Only warn if not already covered by more specific checks
        if warnings.is_empty() {
            warnings.push(ValidationWarning::with_suggestion(
                "Command contains unquoted variable expansion",
                WarningSeverity::Info,
                "Quote variables to prevent word splitting: \"$var\"",
            ));
        }
    }

    if warnings.is_empty() {
        CommandValidationResult::pass(CheckType::DangerousVariable)
    } else {
        CommandValidationResult::pass_with_warnings(CheckType::DangerousVariable, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_command() {
        let result = check_dangerous_variables("echo hello > output.txt");
        assert!(result.allowed);
    }

    #[test]
    fn test_redirect_to_variable() {
        let result = check_dangerous_variables("echo hello > $OUTPUT");
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("redirect target"));
    }

    #[test]
    fn test_redirect_with_braces() {
        let result = check_dangerous_variables("echo hello > ${OUTPUT_FILE}");
        assert!(!result.allowed);
    }

    #[test]
    fn test_append_to_variable() {
        let result = check_dangerous_variables("echo hello >> $LOG");
        assert!(!result.allowed);
    }

    #[test]
    fn test_stderr_to_variable() {
        let result = check_dangerous_variables("cmd 2> $ERR_LOG");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rm_with_variable() {
        let result = check_dangerous_variables("rm -rf $DIR");
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("rm target"));
    }

    #[test]
    fn test_rm_with_braced_variable() {
        let result = check_dangerous_variables("rm ${TEMP_DIR}");
        assert!(!result.allowed);
    }

    #[test]
    fn test_chmod_with_variable() {
        let result = check_dangerous_variables("chmod 755 $FILE");
        assert!(result.allowed); // Just warns
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_unquoted_variable_in_echo() {
        let result = check_dangerous_variables("echo $MESSAGE");
        assert!(result.allowed);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.message.contains("unquoted"))
        );
    }

    #[test]
    fn test_quoted_variable() {
        let result = check_dangerous_variables("echo \"$MESSAGE\"");
        assert!(result.allowed);
        // No warning for quoted variable
    }

    #[test]
    fn test_literal_path() {
        let result = check_dangerous_variables("rm -rf /tmp/test");
        assert!(result.allowed);
    }
}
