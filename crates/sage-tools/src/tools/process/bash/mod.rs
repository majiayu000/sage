//! Bash command execution tool

mod execution;
mod security;
mod types;

pub use security::{requires_user_confirmation, validate_command_security};
pub use types::BashTool;

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use tracing::instrument;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        r#"Executes a given bash command in a persistent shell session with optional timeout, ensuring proper handling and security measures.

IMPORTANT: This tool is for terminal operations like git, npm, docker, etc. DO NOT use it for file operations (reading, writing, editing, searching, finding files) - use the specialized tools for this instead.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If the command will create new directories or files, first use `ls` to verify the parent directory exists and is the correct location
   - For example, before running "mkdir foo/bar", first use `ls foo` to check that "foo" exists and is the intended parent directory

2. Command Execution:
   - Always quote file paths that contain spaces with double quotes (e.g., cd "path with spaces/file.txt")
   - Examples of proper quoting:
     - cd "/Users/name/My Documents" (correct)
     - cd /Users/name/My Documents (incorrect - will fail)
     - python "/path/with spaces/script.py" (correct)
     - python /path/with spaces/script.py (incorrect - will fail)
   - After ensuring proper quoting, execute the command.
   - Capture the output of the command.

Usage notes:
  - The command argument is required.
  - You can specify an optional timeout in milliseconds (up to 600000ms / 10 minutes). If not specified, commands will timeout after 120000ms (2 minutes).
  - It is very helpful if you write a clear, concise description of what this command does in 5-10 words.
  - If the output exceeds 30000 characters, output will be truncated before being returned to you.
  - You can use the `run_in_background` parameter to run the command in the background, which allows you to continue working while the command runs. You can monitor the output using the bash tool as it becomes available. You do not need to use '&' at the end of the command when using this parameter.

  - Avoid using Bash with the `find`, `grep`, `cat`, `head`, `tail`, `sed`, `awk`, or `echo` commands, unless explicitly instructed or when these commands are truly necessary for the task. Instead, always prefer using the dedicated tools for these commands:
    - File search: Use Glob (NOT find or ls)
    - Content search: Use Grep (NOT grep or rg)
    - Read files: Use Read (NOT cat/head/tail)
    - Edit files: Use Edit (NOT sed/awk)
    - Write files: Use Write (NOT echo >/cat <<EOF)
    - Communication: Output text directly (NOT echo/printf)
  - When issuing multiple commands:
    - If the commands are independent and can run in parallel, make multiple bash tool calls in a single message. For example, if you need to run "git status" and "git diff", send a single message with two bash tool calls in parallel.
    - If the commands depend on each other and must run sequentially, use a single bash call with '&&' to chain them together (e.g., `git add . && git commit -m "message" && git push`). For instance, if one operation must complete before another starts (like mkdir before cp, Write before Bash for git operations, or git add before git commit), run these operations sequentially instead.
    - Use ';' only when you need to run commands sequentially but don't care if earlier commands fail
    - DO NOT use newlines to separate commands (newlines are ok in quoted strings)
  - Try to maintain your current working directory throughout the session by using absolute paths and avoiding usage of `cd`. You may use `cd` if the User explicitly requests it.
    <good-example>
    pytest /foo/bar/tests
    </good-example>
    <bad-example>
    cd /foo/bar && pytest tests
    </bad-example>

# Committing changes with git

Only create commits when requested by the user. If unclear, ask first. When the user asks you to create a new git commit, follow these steps carefully:

Git Safety Protocol:
- NEVER update the git config
- NEVER run destructive/irreversible git commands (like push --force, hard reset, etc) unless the user explicitly requests them
- NEVER skip hooks (--no-verify, --no-gpg-sign, etc) unless the user explicitly requests it
- NEVER run force push to main/master, warn the user if they request it
- Avoid git commit --amend. ONLY use --amend when ALL conditions are met:
  (1) User explicitly requested amend, OR commit SUCCEEDED but pre-commit hook auto-modified files that need including
  (2) HEAD commit was created by you in this conversation (verify: git log -1 --format='%an %ae')
  (3) Commit has NOT been pushed to remote (verify: git status shows "Your branch is ahead")
- CRITICAL: If commit FAILED or was REJECTED by hook, NEVER amend - fix the issue and create a NEW commit
- CRITICAL: If you already pushed to remote, NEVER amend unless user explicitly requests it (requires force push)
- NEVER commit changes unless the user explicitly asks you to. It is VERY IMPORTANT to only commit when explicitly asked, otherwise the user will feel that you are being too proactive.

1. Run the following bash commands in parallel:
  - Run a git status command to see all untracked files.
  - Run a git diff command to see both staged and unstaged changes that will be committed.
  - Run a git log command to see recent commit messages, so that you can follow this repository's commit message style.
2. Analyze all staged changes (both previously staged and newly added) and draft a commit message:
  - Summarize the nature of the changes (eg. new feature, enhancement to an existing feature, bug fix, refactoring, test, docs, etc.). Ensure the message accurately reflects the changes and their purpose (i.e. "add" means a wholly new feature, "update" means an enhancement to an existing feature, "fix" means a bug fix, etc.).
  - Do not commit files that likely contain secrets (.env, credentials.json, etc). Warn the user if they specifically request to commit those files
  - Draft a concise (1-2 sentences) commit message that focuses on the "why" rather than the "what"
  - Ensure it accurately reflects the changes and their purpose
3. Run the following commands:
   - Add relevant untracked files to the staging area.
   - Create the commit with a message ending with:
   🤖 Generated with [Sage Agent](https://github.com/sage-agent/sage)
   - Run git status after the commit completes to verify success.
   Note: git status depends on the commit completing, so run it sequentially after the commit.
4. If the commit fails due to pre-commit hook, fix the issue and create a NEW commit (see amend rules above)

Important notes:
- NEVER run additional commands to read or explore code, besides git bash commands
- DO NOT push to the remote repository unless the user explicitly asks you to do so
- IMPORTANT: Never use git commands with the -i flag (like git rebase -i or git add -i) since they require interactive input which is not supported.
- If there are no changes to commit (i.e., no untracked files and no modifications), do not create an empty commit
- In order to ensure good formatting, ALWAYS pass the commit message via a HEREDOC, a la this example:
<example>
git commit -m "$(cat <<'EOF'
   Commit message here.

   🤖 Generated with [Sage Agent](https://github.com/sage-agent/sage)
   EOF
   )"
</example>

# Creating pull requests
Use the gh command via the Bash tool for ALL GitHub-related tasks including working with issues, pull requests, checks, and releases. If given a Github URL use the gh command to get the information needed.

IMPORTANT: When the user asks you to create a pull request, follow these steps carefully:

1. Run the following bash commands in parallel, in order to understand the current state of the branch since it diverged from the main branch:
   - Run a git status command to see all untracked files
   - Run a git diff command to see both staged and unstaged changes that will be committed
   - Check if the current branch tracks a remote branch and is up to date with the remote, so you know if you need to push to the remote
   - Run a git log command and `git diff [base-branch]...HEAD` to understand the full commit history for the current branch (from the time it diverged from the base branch)
2. Analyze all changes that will be included in the pull request, making sure to look at all relevant commits (NOT just the latest commit, but ALL commits that will be included in the pull request!!!), and draft a pull request summary
3. Run the following commands in parallel:
   - Create new branch if needed
   - Push to remote with -u flag if needed
   - Create PR using gh pr create with the format below. Use a HEREDOC to pass the body to ensure correct formatting.
<example>
gh pr create --title "the pr title" --body "$(cat <<'EOF'
## Summary
<1-3 bullet points>

## Test plan
[Bulleted markdown checklist of TODOs for testing the pull request...]

🤖 Generated with [Sage Agent](https://github.com/sage-agent/sage)
EOF
)"
</example>

Important:
- Return the PR URL when you're done, so the user can see it

# Other common operations
- View comments on a Github PR: gh api repos/foo/bar/pulls/123/comments"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("command", "The bash command to execute"),
                ToolParameter::boolean(
                    "run_in_background",
                    "If true, run command in background (default: false)",
                )
                .optional()
                .with_default(false),
                ToolParameter::optional_string(
                    "shell_id",
                    "Optional custom ID for background shell (auto-generated if not provided)",
                ),
                ToolParameter::boolean(
                    "user_confirmed",
                    "Set to true ONLY after getting explicit user confirmation via ask_user_question tool for destructive commands (rm, rmdir, git push --force, etc.)",
                )
                .optional()
                .with_default(false),
            ],
        )
    }

    #[instrument(skip(self, call), fields(call_id = %call.id, run_in_background))]
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        // Check if this is a destructive command that requires user confirmation
        // The agent must explicitly acknowledge by setting user_confirmed=true
        let user_confirmed = call.get_bool("user_confirmed").unwrap_or(false);
        if let Some(reason) = requires_user_confirmation(&command) {
            if !user_confirmed {
                return Err(ToolError::ConfirmationRequired(format!(
                    "⚠️  DESTRUCTIVE COMMAND BLOCKED\n\n\
                    {}\n\n\
                    Before executing this command, you MUST:\n\
                    1. Use the ask_user_question tool to get explicit user confirmation\n\
                    2. Wait for the user's response\n\
                    3. Only if user confirms, call this tool again with user_confirmed=true\n\n\
                    DO NOT proceed without user confirmation!",
                    reason
                )));
            }
            tracing::info!(
                command = %command,
                "executing confirmed destructive command"
            );
        }

        let run_in_background = call.get_bool("run_in_background").unwrap_or(false);
        tracing::Span::current().record("run_in_background", run_in_background);

        let shell_id = call.get_string("shell_id");

        tracing::debug!(
            command_preview = %command.chars().take(100).collect::<String>(),
            "executing bash command"
        );

        let mut result = if run_in_background {
            self.execute_background(&command, shell_id).await?
        } else {
            self.execute_command(&command).await?
        };

        if result.success {
            tracing::info!("bash command completed successfully");
        } else {
            tracing::warn!("bash command failed");
        }

        result.call_id = call.id.clone();
        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        // Validate the command for security issues
        validate_command_security(&command)?;

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(300) // 5 minutes
    }

    fn supports_parallel_execution(&self) -> bool {
        false // Commands might interfere with each other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_bash_tool_simple_command() {
        let tool = BashTool::new();
        let call = create_tool_call(
            "test-1",
            "bash",
            json!({
                "command": "echo 'Hello, World!'"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_bash_tool_pwd_command() {
        let tool = BashTool::new();
        let call = create_tool_call(
            "test-2",
            "bash",
            json!({
                "command": "pwd"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_invalid_command() {
        let tool = BashTool::new();
        let call = create_tool_call(
            "test-3",
            "bash",
            json!({
                "command": "nonexistent_command_12345"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_with_working_directory() {
        let temp_dir = std::env::temp_dir();
        let tool = BashTool::with_working_directory(&temp_dir);
        let call = create_tool_call(
            "test-4",
            "bash",
            json!({
                "command": "pwd"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        // Just verify we got some output - temp dir paths may differ after canonicalization
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_bash_tool_missing_command() {
        let tool = BashTool::new();
        let call = create_tool_call("test-5", "bash", json!({}));

        // Implementation returns Err for missing parameters
        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing") || err.to_string().contains("command"));
    }

    #[tokio::test]
    async fn test_bash_tool_allowed_commands() {
        let tool =
            BashTool::new().with_allowed_commands(vec!["echo".to_string(), "pwd".to_string()]);

        // Test allowed command
        let call = create_tool_call(
            "test-6a",
            "bash",
            json!({
                "command": "echo 'allowed'"
            }),
        );
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Test disallowed command - returns Err
        let call = create_tool_call(
            "test-6b",
            "bash",
            json!({
                "command": "ls"
            }),
        );
        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not allowed") || err.to_string().contains("Command"));
    }

    #[test]
    fn test_bash_tool_schema() {
        let tool = BashTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "bash");
        assert!(!schema.description.is_empty());
    }

    // Security validation tests
    #[test]
    fn test_dangerous_commands_blocked() {
        // Test dangerous command patterns are blocked
        let dangerous_commands = vec![
            "rm -rf /",
            "rm -rf /*",
            ":(){ :|:& };:",
            "dd if=/dev/zero of=/dev/sda",
            "mkfs.ext4 /dev/sda",
            "shutdown -h now",
            "reboot",
        ];

        for cmd in dangerous_commands {
            let result = validate_command_security(cmd);
            assert!(result.is_err(), "Command should be blocked: {}", cmd);
        }
    }

    #[test]
    fn test_privilege_escalation_blocked() {
        // Test privilege escalation commands are blocked
        let priv_commands = vec![
            "sudo rm -rf /tmp/test",
            "su - root",
            "doas ls",
            "pkexec /bin/bash",
        ];

        for cmd in priv_commands {
            let result = validate_command_security(cmd);
            assert!(result.is_err(), "Command should be blocked: {}", cmd);
        }
    }

    #[test]
    fn test_safe_commands_allowed() {
        // Test that safe commands are allowed
        let safe_commands = vec![
            "echo 'Hello, World!'",
            "ls -la",
            "pwd",
            "cat file.txt",
            "grep pattern file.txt",
            "find . -name '*.rs'",
            "head -n 10 file.txt",
            "tail -f log.txt",
            "wc -l file.txt",
        ];

        for cmd in safe_commands {
            let result = validate_command_security(cmd);
            assert!(result.is_ok(), "Command should be allowed: {}", cmd);
        }
    }

    #[test]
    fn test_command_chaining_allowed() {
        // Test that command chaining is now allowed
        let chained_commands = vec![
            "cd /tmp && ls",
            "echo hello; echo world",
            "false || echo 'failed'",
            "cd /repo && python -c 'import sys; print(sys.version)'",
        ];

        for cmd in chained_commands {
            let result = validate_command_security(cmd);
            assert!(
                result.is_ok(),
                "Command chaining should be allowed: {}",
                cmd
            );
        }
    }

    #[test]
    fn test_command_substitution_allowed() {
        // Test that command substitution is now allowed
        let subst_commands = vec!["echo $(pwd)", "echo `date`", "echo ${HOME}"];

        for cmd in subst_commands {
            let result = validate_command_security(cmd);
            assert!(
                result.is_ok(),
                "Command substitution should be allowed: {}",
                cmd
            );
        }
    }

    #[test]
    fn test_pipe_and_redirect_allowed() {
        // Test that pipes and redirects are allowed
        let pipe_commands = vec![
            "ls | head -10",
            "grep pattern file.txt | wc -l",
            "echo 'test' > output.txt",
            "cat file.txt >> output.txt",
        ];

        for cmd in pipe_commands {
            let result = validate_command_security(cmd);
            assert!(result.is_ok(), "Command should be allowed: {}", cmd);
        }
    }

    #[test]
    fn test_chained_dangerous_still_blocked() {
        // Even with chaining allowed, dangerous commands are still blocked
        let dangerous_chained = vec![
            "echo hello && rm -rf /",
            "ls; sudo rm -rf /tmp",
            "false || shutdown -h now",
        ];

        for cmd in dangerous_chained {
            let result = validate_command_security(cmd);
            assert!(
                result.is_err(),
                "Dangerous command should still be blocked: {}",
                cmd
            );
        }
    }
}
