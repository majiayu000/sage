//! Heredoc injection safety check following Claude Code patterns.
//!
//! Detects potential heredoc injection attacks where:
//! - Delimiter contains variable references ($VAR)
//! - Delimiter contains command substitution ($(cmd) or `cmd`)
//! - Unquoted delimiters that could be manipulated

use super::types::{CheckType, CommandValidationResult, ValidationWarning, WarningSeverity};

/// Parsed heredoc information
struct HeredocInfo {
    /// The quote character used (empty string if none)
    quote: String,
    /// The delimiter string
    delimiter: String,
}

/// Parse heredoc patterns from command
///
/// Matches: << DELIMITER, <<- DELIMITER, << 'DELIMITER', << "DELIMITER"
fn find_heredocs(command: &str) -> Vec<HeredocInfo> {
    let mut result = Vec::new();
    let chars: Vec<char> = command.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Look for <<
        if i + 1 < len && chars[i] == '<' && chars[i + 1] == '<' {
            i += 2;

            // Skip optional - (for <<-)
            if i < len && chars[i] == '-' {
                i += 1;
            }

            // Skip whitespace
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }

            if i >= len {
                break;
            }

            // Check for quote character
            let quote = if chars[i] == '\'' || chars[i] == '"' {
                let q = chars[i];
                i += 1;
                q
            } else {
                '\0'
            };

            // Read delimiter
            let start = i;
            if quote != '\0' {
                // Read until closing quote
                while i < len && chars[i] != quote {
                    i += 1;
                }
            } else {
                // Read until whitespace
                while i < len && !chars[i].is_whitespace() {
                    i += 1;
                }
            }

            let delimiter: String = chars[start..i].iter().collect();
            if !delimiter.is_empty() {
                result.push(HeredocInfo {
                    quote: if quote != '\0' {
                        quote.to_string()
                    } else {
                        String::new()
                    },
                    delimiter,
                });
            }
        }
        i += 1;
    }

    result
}

/// Check if string contains variable references ($VAR, ${VAR})
fn has_variable_reference(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == '$' {
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                // Check for ${VAR} or $VAR
                if next == '{' || next.is_alphabetic() || next == '_' {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if string contains command substitution ($(...) or `...`)
fn has_command_substitution(s: &str) -> bool {
    // Check for $(...)
    if s.contains("$(") {
        return true;
    }
    // Check for `...`
    if s.contains('`') {
        return true;
    }
    false
}

/// Check heredoc delimiter safety
///
/// Returns a blocking result if the heredoc delimiter contains:
/// - Variable references ($VAR, ${VAR})
/// - Command substitution ($(cmd), `cmd`)
/// - Suspicious patterns that could lead to injection
pub fn check_heredoc_safety(command: &str) -> CommandValidationResult {
    let mut warnings = Vec::new();

    // Find all heredoc patterns
    for heredoc in find_heredocs(command) {
        let delimiter = &heredoc.delimiter;

        // Check for variable in delimiter
        if has_variable_reference(delimiter) {
            return CommandValidationResult::block(
                CheckType::Heredoc,
                format!(
                    "Heredoc delimiter '{}' contains variable reference - potential injection",
                    delimiter
                ),
            );
        }

        // Check for command substitution in delimiter
        if has_command_substitution(delimiter) {
            return CommandValidationResult::block(
                CheckType::Heredoc,
                format!(
                    "Heredoc delimiter '{}' contains command substitution - potential injection",
                    delimiter
                ),
            );
        }

        // Warn about unquoted delimiters (variables in content will expand)
        if heredoc.quote.is_empty() && !delimiter.starts_with('\'') {
            warnings.push(ValidationWarning::with_suggestion(
                format!(
                    "Unquoted heredoc delimiter '{}' allows variable expansion",
                    delimiter
                ),
                WarningSeverity::Warning,
                format!("Use quoted delimiter: << '{}'", delimiter),
            ));
        }
    }

    if warnings.is_empty() {
        CommandValidationResult::pass(CheckType::Heredoc)
    } else {
        CommandValidationResult::pass_with_warnings(CheckType::Heredoc, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_heredoc() {
        let result = check_heredoc_safety("cat << 'EOF'\nhello\nEOF");
        assert!(result.allowed);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_unquoted_heredoc_warning() {
        let result = check_heredoc_safety("cat << EOF\nhello\nEOF");
        assert!(result.allowed);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_variable_in_delimiter() {
        let result = check_heredoc_safety("cat << $DELIM\nhello\n$DELIM");
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("variable reference"));
    }

    #[test]
    fn test_command_subst_in_delimiter() {
        let result = check_heredoc_safety("cat << $(echo EOF)\nhello\nEOF");
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("command substitution"));
    }

    #[test]
    fn test_backtick_in_delimiter() {
        let result = check_heredoc_safety("cat << `echo EOF`\nhello\nEOF");
        assert!(!result.allowed);
    }

    #[test]
    fn test_no_heredoc() {
        let result = check_heredoc_safety("echo hello");
        assert!(result.allowed);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_heredoc_with_dash() {
        let result = check_heredoc_safety("cat <<- 'EOF'\n\thello\n\tEOF");
        assert!(result.allowed);
    }
}
