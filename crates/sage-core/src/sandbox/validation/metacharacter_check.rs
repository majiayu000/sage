//! Shell metacharacter validation following Claude Code patterns.
//!
//! Detects potentially dangerous shell metacharacters:
//! - Command chaining: ; && ||
//! - Piping: |
//! - Backgrounding: &
//! - Subshells: () $()

use super::types::{
    CheckType, ValidationContext, ValidationResult, ValidationWarning, WarningSeverity,
};

/// Pattern to detect subshell execution
fn has_subshell(command: &str) -> bool {
    // Check for $(...) pattern
    let mut depth = 0;
    let chars: Vec<char> = command.chars().collect();
    for i in 0..chars.len() {
        if i > 0 && chars[i - 1] == '$' && chars[i] == '(' {
            return true;
        }
        // Also check for standalone (...) not preceded by $
        if chars[i] == '(' {
            depth += 1;
        } else if chars[i] == ')' && depth > 0 {
            depth -= 1;
        }
    }
    // Check for standalone parentheses (simple heuristic)
    command.contains("$(") || (command.contains('(') && command.contains(')'))
}

/// Check if command contains pipe (|) but not logical OR (||)
fn has_pipe(command: &str) -> bool {
    let chars: Vec<char> = command.chars().collect();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for i in 0..chars.len() {
        let c = chars[i];

        // Track quote state
        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }

        // Skip if inside quotes
        if in_single_quote || in_double_quote {
            continue;
        }

        // Check for single pipe (not ||)
        if c == '|' {
            let next = chars.get(i + 1);
            if next != Some(&'|') {
                // Check previous char is also not |
                if i == 0 || chars[i - 1] != '|' {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if command contains background operator (&) but not logical AND (&&)
fn has_background(command: &str) -> bool {
    let chars: Vec<char> = command.chars().collect();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for i in 0..chars.len() {
        let c = chars[i];

        // Track quote state
        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }

        // Skip if inside quotes
        if in_single_quote || in_double_quote {
            continue;
        }

        // Check for single & (not &&)
        if c == '&' {
            let next = chars.get(i + 1);
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };

            // Not && (neither preceded nor followed by &)
            if next != Some(&'&') && prev != Some('&') {
                return true;
            }
        }
    }
    false
}

/// Check if command contains command separators outside quotes
fn has_command_separator(command: &str) -> bool {
    let chars: Vec<char> = command.chars().collect();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for i in 0..chars.len() {
        let c = chars[i];

        // Track quote state
        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }

        // Skip if inside quotes
        if in_single_quote || in_double_quote {
            continue;
        }

        // Check for semicolon
        if c == ';' {
            return true;
        }

        // Check for && or ||
        if c == '&' && chars.get(i + 1) == Some(&'&') {
            return true;
        }
        if c == '|' && chars.get(i + 1) == Some(&'|') {
            return true;
        }
    }
    false
}

/// Check for dangerous shell metacharacters
///
/// Based on context settings, may block or warn about:
/// - Command chaining (;, &&, ||)
/// - Piping (|)
/// - Background execution (&)
/// - Subshell execution
pub fn check_shell_metacharacters(command: &str, context: &ValidationContext) -> ValidationResult {
    let mut warnings = Vec::new();

    // Check for command separators
    if has_command_separator(command) {
        if !context.allow_chaining {
            return ValidationResult::block(
                CheckType::ShellMetacharacter,
                "Command chaining with ; && || is not allowed in strict mode",
            );
        }
        warnings.push(ValidationWarning::info(
            "Command contains chaining operators (;, &&, ||)",
        ));
    }

    // Check for pipes
    if has_pipe(command) {
        warnings.push(ValidationWarning::new(
            "Command contains pipe operator",
            WarningSeverity::Info,
        ));
    }

    // Check for background execution
    if has_background(command) {
        if !context.allow_background {
            return ValidationResult::block(
                CheckType::ShellMetacharacter,
                "Background execution (&) is not allowed in strict mode",
            );
        }
        warnings.push(ValidationWarning::warning(
            "Command will run in background - output may be delayed",
        ));
    }

    // Check for subshell
    if has_subshell(command) {
        warnings.push(ValidationWarning::new(
            "Command contains subshell execution",
            WarningSeverity::Info,
        ));
    }

    if warnings.is_empty() {
        ValidationResult::pass(CheckType::ShellMetacharacter)
    } else {
        ValidationResult::pass_with_warnings(CheckType::ShellMetacharacter, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let ctx = ValidationContext::default();
        let result = check_shell_metacharacters("ls -la", &ctx);
        assert!(result.allowed);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_semicolon_strict() {
        let ctx = ValidationContext::strict();
        let result = check_shell_metacharacters("echo a; echo b", &ctx);
        assert!(!result.allowed);
    }

    #[test]
    fn test_semicolon_permissive() {
        let ctx = ValidationContext::permissive();
        let result = check_shell_metacharacters("echo a; echo b", &ctx);
        assert!(result.allowed);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_and_chain() {
        let ctx = ValidationContext::strict();
        let result = check_shell_metacharacters("mkdir dir && cd dir", &ctx);
        assert!(!result.allowed);
    }

    #[test]
    fn test_or_chain() {
        let ctx = ValidationContext::strict();
        let result = check_shell_metacharacters("test -f file || touch file", &ctx);
        assert!(!result.allowed);
    }

    #[test]
    fn test_pipe() {
        let ctx = ValidationContext::default();
        let result = check_shell_metacharacters("cat file | grep pattern", &ctx);
        assert!(result.allowed);
        // Pipe generates info warning
        assert!(result.warnings.iter().any(|w| w.message.contains("pipe")));
    }

    #[test]
    fn test_background_strict() {
        let ctx = ValidationContext::strict();
        let result = check_shell_metacharacters("sleep 10 &", &ctx);
        assert!(!result.allowed);
    }

    #[test]
    fn test_background_permissive() {
        let ctx = ValidationContext::permissive();
        let result = check_shell_metacharacters("sleep 10 &", &ctx);
        assert!(result.allowed);
    }

    #[test]
    fn test_quoted_semicolon() {
        let ctx = ValidationContext::strict();
        // Semicolon inside quotes should not trigger
        let result = check_shell_metacharacters("echo 'a; b'", &ctx);
        assert!(result.allowed);
    }

    #[test]
    fn test_subshell() {
        let ctx = ValidationContext::default();
        let result = check_shell_metacharacters("echo $(date)", &ctx);
        assert!(result.allowed);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.message.contains("subshell"))
        );
    }
}
