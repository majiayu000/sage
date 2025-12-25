//! Command security validation

use sage_core::tools::base::ToolError;

/// Validate command for security issues
///
/// This checks for:
/// - Dangerous command patterns (system destruction, privilege escalation)
/// - Fork bombs and disk destruction commands
///
/// Note: Command chaining (&&, ;, ||) and command substitution ($(), ``) are allowed
/// as they are commonly needed for development workflows.
pub fn validate_command_security(command: &str) -> Result<(), ToolError> {
    let command_lower = command.to_lowercase();

    // Dangerous command patterns - system destruction
    let dangerous_commands = [
        "rm -rf /",
        "rm -rf /*",
        "rm -rf ~",
        ":(){ :|:& };:", // Fork bomb
        ":(){:|:&};:",   // Fork bomb variant (no spaces)
        "dd if=/dev/zero of=/dev/sda",
        "dd if=/dev/random of=/dev/sda",
        "> /dev/sda",
        "mv /* /dev/null",
        "chmod -r 000 /",
        "mkfs",
        "fdisk",
        "parted",
        "shutdown",
        "reboot",
        "halt",
        "poweroff",
        "init 0",
        "init 6",
        "telinit 0",
    ];

    for pattern in &dangerous_commands {
        if command_lower.contains(pattern) {
            return Err(ToolError::PermissionDenied(format!(
                "Dangerous command pattern detected: {}",
                pattern
            )));
        }
    }

    // Privilege escalation commands
    let privilege_commands = ["sudo ", "su ", "doas ", "pkexec "];
    for pattern in &privilege_commands {
        if command_lower.starts_with(pattern)
            || command_lower.contains(&format!(" {}", pattern.trim()))
        {
            return Err(ToolError::PermissionDenied(format!(
                "Privilege escalation command not allowed: {}",
                pattern.trim()
            )));
        }
    }

    // Note: We now allow:
    // - Command chaining: &&, ;, || (commonly needed for development)
    // - Command substitution: $(), `` (commonly needed for scripting)
    // - Variable expansion: ${} (commonly needed)
    // - Pipes and redirects: |, >, < (always allowed)

    Ok(())
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
        return Some(format!(
            "This command will truncate a table: '{}'",
            command
        ));
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
