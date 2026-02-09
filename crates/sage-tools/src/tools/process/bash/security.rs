//! Command security validation following Claude Code patterns.
//!
//! This module provides comprehensive security validation:
//! - Heredoc injection detection
//! - Shell metacharacter checks
//! - Variable injection detection
//! - Dangerous pattern detection
//! - Critical path removal prevention

use sage_core::sandbox::validation::{
    CheckType, ValidationContext, ValidationWarning, validate_command,
};
use sage_core::sandbox::violations::{SharedViolationStore, Violation, ViolationType};
use sage_core::tools::base::ToolError;

/// Validate command for security issues (legacy API)
///
/// This checks for:
/// - Dangerous command patterns (system destruction, privilege escalation)
/// - Fork bombs and disk destruction commands
///
/// Note: Command chaining (&&, ;, ||) and command substitution ($(), ``) are allowed
/// as they are commonly needed for development workflows.
pub fn validate_command_security(command: &str) -> Result<(), ToolError> {
    validate_command_comprehensive(command, None)
}

/// Comprehensive command validation following Claude Code patterns
///
/// Performs all security checks and optionally records violations.
/// Returns warnings for non-blocking issues.
pub fn validate_command_comprehensive(
    command: &str,
    violation_store: Option<&SharedViolationStore>,
) -> Result<(), ToolError> {
    let command_lower = command.to_lowercase();

    // Fork bomb detection (not in standard validation)
    let fork_bombs = [
        ":(){ :|:& };:", // Fork bomb
        ":(){:|:&};:",   // Fork bomb variant (no spaces)
    ];
    for pattern in &fork_bombs {
        if command_lower.contains(pattern) {
            if let Some(store) = violation_store {
                store.record(Violation::blocked(
                    ViolationType::DangerousPattern,
                    "Fork bomb detected",
                    command,
                ));
            }
            return Err(ToolError::PermissionDenied(
                "Fork bomb detected - command blocked".to_string(),
            ));
        }
    }

    // System destruction commands (beyond what removal_check catches)
    let system_destruction = [
        "dd if=/dev/zero of=/dev/sda",
        "dd if=/dev/random of=/dev/sda",
        "> /dev/sda",
        "mv /* /dev/null",
        "chmod -r 000 /",
        "shutdown",
        "reboot",
        "halt",
        "poweroff",
        "init 0",
        "init 6",
        "telinit 0",
    ];
    for pattern in &system_destruction {
        if command_lower.contains(pattern) {
            if let Some(store) = violation_store {
                store.record(Violation::blocked(
                    ViolationType::DangerousPattern,
                    format!("System destruction command: {}", pattern),
                    command,
                ));
            }
            return Err(ToolError::PermissionDenied(format!(
                "Dangerous command pattern detected: {}",
                pattern
            )));
        }
    }

    // Use the validation module for comprehensive checks
    let context = ValidationContext::permissive();
    let result = validate_command(command, &context);

    if !result.allowed {
        // Record the violation if we have a store
        if let Some(store) = violation_store {
            let violation_type = match result.check_type {
                CheckType::Heredoc => ViolationType::HeredocInjection,
                CheckType::ShellMetacharacter => ViolationType::ShellMetacharacterAbuse,
                CheckType::DangerousVariable => ViolationType::VariableInjection,
                CheckType::DangerousPattern => ViolationType::DangerousPattern,
                CheckType::DangerousRemoval => ViolationType::CriticalPathRemoval,
                CheckType::Composite => ViolationType::CommandBlocked,
            };
            store.record(Violation::blocked(
                violation_type,
                result.reason.as_deref().unwrap_or("Command blocked"),
                command,
            ));
        }

        return Err(ToolError::PermissionDenied(format!(
            "Command blocked by {} check: {}",
            result.check_type.as_str(),
            result.reason.unwrap_or_default()
        )));
    }

    // Log warnings if we have a store
    if let Some(store) = violation_store {
        for warning in &result.warnings {
            if warning.severity == sage_core::sandbox::validation::WarningSeverity::Critical {
                store.record(
                    Violation::warning(ViolationType::DangerousPattern, &warning.message, command)
                        .with_context(warning.suggestion.clone().unwrap_or_default()),
                );
            }
        }
    }

    Ok(())
}

/// Validate command with strictness level
pub fn validate_command_with_strictness(
    command: &str,
    strict: bool,
    violation_store: Option<&SharedViolationStore>,
) -> Result<Vec<ValidationWarning>, ToolError> {
    let context = if strict {
        ValidationContext::strict()
    } else {
        ValidationContext::permissive()
    };

    let result = validate_command(command, &context);

    if !result.allowed {
        if let Some(store) = violation_store {
            let violation_type = match result.check_type {
                CheckType::Heredoc => ViolationType::HeredocInjection,
                CheckType::ShellMetacharacter => ViolationType::ShellMetacharacterAbuse,
                CheckType::DangerousVariable => ViolationType::VariableInjection,
                CheckType::DangerousPattern => ViolationType::DangerousPattern,
                CheckType::DangerousRemoval => ViolationType::CriticalPathRemoval,
                CheckType::Composite => ViolationType::CommandBlocked,
            };
            store.record(Violation::blocked(
                violation_type,
                result.reason.as_deref().unwrap_or("Command blocked"),
                command,
            ));
        }

        return Err(ToolError::PermissionDenied(format!(
            "Command blocked: {}",
            result.reason.unwrap_or_default()
        )));
    }

    Ok(result.warnings)
}

/// Check if a command is destructive and requires user confirmation
///
/// Returns Some(reason) if confirmation is required, None otherwise
pub fn requires_user_confirmation(command: &str) -> Option<String> {
    let command_lower = command.to_lowercase();
    let command_trimmed = command_lower.trim();

    // rm command - file/directory deletion
    if command_trimmed.starts_with("rm ") || command_trimmed == "rm" {
        // rm -rf is especially dangerous
        if command_lower.contains("-rf") || command_lower.contains("-r") {
            return Some(format!(
                "This command will recursively delete files/directories: '{}'",
                command
            ));
        }
        return Some(format!("This command will delete files: '{}'", command));
    }

    // rmdir - directory deletion
    if command_trimmed.starts_with("rmdir ") {
        return Some(format!(
            "This command will delete directories: '{}'",
            command
        ));
    }

    // git push --force
    if command_lower.contains("git") && command_lower.contains("push") {
        if command_lower.contains("--force") || command_lower.contains("-f") {
            return Some(format!(
                "This command will force push, potentially overwriting remote history: '{}'",
                command
            ));
        }
    }

    // git reset --hard
    if command_lower.contains("git")
        && command_lower.contains("reset")
        && command_lower.contains("--hard")
    {
        return Some(format!(
            "This command will discard all local changes: '{}'",
            command
        ));
    }

    // DROP DATABASE / DROP TABLE
    if command_lower.contains("drop database") || command_lower.contains("drop table") {
        return Some(format!(
            "This command will drop database objects: '{}'",
            command
        ));
    }

    // truncate / delete from without where
    if command_lower.contains("truncate ") {
        return Some(format!("This command will truncate a table: '{}'", command));
    }

    // docker system prune
    if command_lower.contains("docker")
        && (command_lower.contains("prune") || command_lower.contains("rm"))
    {
        return Some(format!(
            "This command will remove Docker resources: '{}'",
            command
        ));
    }

    None
}
