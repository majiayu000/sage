//! Command validation module following Claude Code patterns.
//!
//! This module provides comprehensive command validation to prevent:
//! - Heredoc injection attacks
//! - Shell metacharacter abuse
//! - Variable injection in redirects
//! - Dangerous command patterns
//! - Critical path removal

mod heredoc_check;
mod metacharacter_check;
mod pattern_check;
mod removal_check;
mod types;
mod variable_check;

pub use heredoc_check::check_heredoc_safety;
pub use metacharacter_check::check_shell_metacharacters;
pub use pattern_check::check_dangerous_patterns;
pub use removal_check::check_dangerous_removal;
pub use types::{
    CheckType, ValidationContext, ValidationResult, ValidationWarning, WarningSeverity,
};
pub use variable_check::check_dangerous_variables;

use crate::tools::ToolError;

/// Perform comprehensive command validation
///
/// Runs all validation checks and returns a combined result.
pub fn validate_command(command: &str, context: &ValidationContext) -> ValidationResult {
    let checks = [
        check_heredoc_safety(command),
        check_shell_metacharacters(command, context),
        check_dangerous_variables(command),
        check_dangerous_patterns(command),
        check_dangerous_removal(command),
    ];

    let mut all_warnings = Vec::new();

    for result in &checks {
        if !result.allowed {
            // Return first blocking result with all accumulated warnings
            let mut blocking = result.clone();
            blocking.warnings.extend(all_warnings);
            blocking.check_type = CheckType::Composite;
            return blocking;
        }
        all_warnings.extend(result.warnings.clone());
    }

    ValidationResult::pass_with_warnings(CheckType::Composite, all_warnings)
}

/// Validate command and return Result for tool integration
pub fn validate_command_for_tool(
    command: &str,
    context: &ValidationContext,
) -> Result<Vec<ValidationWarning>, ToolError> {
    let result = validate_command(command, context);

    if result.allowed {
        Ok(result.warnings)
    } else {
        Err(ToolError::ExecutionFailed(format!(
            "Command blocked by {} check: {}",
            result.check_type.as_str(),
            result.reason.unwrap_or_default()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_safe_command() {
        let ctx = ValidationContext::permissive();
        let result = validate_command("ls -la", &ctx);
        assert!(result.allowed);
    }

    #[test]
    fn test_validate_dangerous_removal() {
        let ctx = ValidationContext::strict();
        let result = validate_command("rm -rf /", &ctx);
        assert!(!result.allowed);
    }

    #[test]
    fn test_validate_heredoc_injection() {
        let ctx = ValidationContext::default();
        let result = validate_command("cat << $EOF\nmalicious\n$EOF", &ctx);
        assert!(!result.allowed);
    }
}
