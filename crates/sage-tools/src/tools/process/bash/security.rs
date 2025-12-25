//! Command security validation

use sage_core::tools::base::ToolError;

/// Validate command for security issues
///
/// This checks for:
/// - Dangerous command patterns (system destruction, privilege escalation)
/// - Shell operators that could enable command injection
/// - Command substitution attempts
pub fn validate_command_security(command: &str) -> Result<(), ToolError> {
    let command_lower = command.to_lowercase();

    // Dangerous command patterns - system destruction
    let dangerous_commands = [
        "rm -rf /",
        "rm -rf /*",
        "rm -rf ~",
        ":(){ :|:& };:", // Fork bomb
        ":(){:|:&};:",   // Fork bomb variant (no spaces)
        "dd if=/dev/zero",
        "dd if=/dev/random",
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
        "> /dev/sda",
        "mv /* /dev/null",
        "chmod -r 000 /",
        "chown -r",
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

    // Check for command substitution which could bypass validation
    // These allow executing arbitrary commands within the main command
    let substitution_patterns = [
        "$(", // Modern command substitution
        "`",  // Legacy command substitution (backticks)
        "${", // Variable expansion with commands
    ];

    for pattern in &substitution_patterns {
        if command.contains(pattern) {
            return Err(ToolError::PermissionDenied(format!(
                "Command substitution not allowed: {}",
                pattern
            )));
        }
    }

    // Check for dangerous shell operators that enable command chaining
    // Note: We allow pipes (|) and redirects (>, <) as they are commonly needed
    // but block command separators that could run arbitrary commands
    let dangerous_operators = [
        ";",  // Command separator - runs multiple commands
        "&&", // Logical AND - runs second command if first succeeds
        "||", // Logical OR - runs second command if first fails
    ];

    for op in &dangerous_operators {
        if command.contains(op) {
            return Err(ToolError::PermissionDenied(format!(
                "Command chaining operator not allowed: '{}'",
                op
            )));
        }
    }

    // Check for process backgrounding which could escape control
    if command.trim().ends_with('&') && !command.trim().ends_with("&&") {
        return Err(ToolError::PermissionDenied(
            "Background process operator (&) not allowed at end of command".to_string(),
        ));
    }

    Ok(())
}

/// Permissive command security validation for isolated environments (SWE-bench, Docker, etc.)
///
/// This allows command chaining operators (&&, ;, ||) and command substitution,
/// but still blocks the most dangerous commands that could destroy the system.
pub fn validate_command_security_permissive(command: &str) -> Result<(), ToolError> {
    let command_lower = command.to_lowercase();

    // Even in permissive mode, block absolutely dangerous commands
    let dangerous_commands = [
        "rm -rf /",
        "rm -rf /*",
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
    ];

    for pattern in &dangerous_commands {
        if command_lower.contains(pattern) {
            return Err(ToolError::PermissionDenied(format!(
                "Dangerous command pattern detected: {}",
                pattern
            )));
        }
    }

    // Block privilege escalation even in permissive mode
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

    // In permissive mode: Allow &&, ;, ||, $(), `, ${}
    // These are commonly needed for development workflows

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
