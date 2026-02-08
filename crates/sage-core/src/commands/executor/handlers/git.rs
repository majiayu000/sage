//! Git-related command handlers
//!
//! Provides commands for git operations:
//! - /commit - AI-assisted commit message generation
//! - /review-pr - AI-assisted PR review

use crate::commands::types::{CommandInvocation, CommandResult};
use crate::error::SageResult;
use std::process::Command;

/// Execute /commit command - generate commit message from staged changes
pub async fn execute_commit(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    // Check if we're in a git repository
    let git_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output();

    if git_check.is_err() || !git_check.unwrap().status.success() {
        return Ok(CommandResult::local(
            "Not in a git repository. Please run this command from within a git repository.",
        ));
    }

    // Get staged changes
    let diff_output = Command::new("git")
        .args(["diff", "--cached", "--stat"])
        .output();

    let diff_stat = match diff_output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(e) => {
            return Ok(CommandResult::local(format!(
                "Failed to get staged changes: {}",
                e
            )));
        }
    };

    if diff_stat.trim().is_empty() {
        return Ok(CommandResult::local(
            "No staged changes found.\n\nTo stage changes:\n  git add <files>\n  git add -A  (stage all)\n\nThen run /commit again.",
        ));
    }

    // Get detailed diff for context
    let diff_detail = Command::new("git")
        .args(["diff", "--cached"])
        .output();

    let diff_content = match diff_detail {
        Ok(output) => {
            let content = String::from_utf8_lossy(&output.stdout).to_string();
            // Truncate if too long
            if content.len() > 10000 {
                format!("{}...\n\n[Diff truncated, {} more characters]", &content[..10000], content.len() - 10000)
            } else {
                content
            }
        }
        Err(_) => String::new(),
    };

    // Get recent commit messages for style reference
    let log_output = Command::new("git")
        .args(["log", "--oneline", "-10"])
        .output();

    let recent_commits = match log_output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => String::new(),
    };

    // Check if user provided a message hint
    let message_hint = if !invocation.arguments.is_empty() {
        format!("\n\nUser's hint for the commit: {}", invocation.arguments.join(" "))
    } else {
        String::new()
    };

    // Build prompt for LLM
    let prompt = format!(
        r#"Please help me create a git commit message for the following staged changes.

## Staged Changes Summary
```
{}
```

## Detailed Diff
```diff
{}
```

## Recent Commit Messages (for style reference)
```
{}
```
{}

## Instructions
1. Analyze the changes and understand what was modified
2. Generate a commit message following these guidelines:
   - Use conventional commits format if the project uses it (feat:, fix:, docs:, etc.)
   - First line should be a concise summary (50 chars or less ideally)
   - If needed, add a blank line followed by a more detailed description
   - Focus on WHY the change was made, not just WHAT changed
3. Match the style of recent commits if there's a clear pattern

Please provide the commit message, then I'll help you commit it."#,
        diff_stat,
        diff_content,
        recent_commits,
        message_hint
    );

    Ok(CommandResult::prompt(prompt)
        .with_status("Analyzing staged changes...")
        .with_tool_restrictions(vec![
            "Bash".to_string(),
            "Read".to_string(),
        ]))
}

/// Execute /review-pr command - review a pull request
pub async fn execute_review_pr(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    // Check if we're in a git repository
    let git_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output();

    if git_check.is_err() || !git_check.unwrap().status.success() {
        return Ok(CommandResult::local(
            "Not in a git repository. Please run this command from within a git repository.",
        ));
    }

    // Get PR number or branch from arguments
    let pr_ref = if !invocation.arguments.is_empty() {
        invocation.arguments[0].clone()
    } else {
        // Try to get current branch
        let branch_output = Command::new("git")
            .args(["branch", "--show-current"])
            .output();

        match branch_output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(_) => {
                return Ok(CommandResult::local(
                    "Usage: /review-pr [pr-number|branch]\n\nExamples:\n  /review-pr 123\n  /review-pr feature/my-branch",
                ));
            }
        }
    };

    // Check if it's a PR number or branch name
    let is_pr_number = pr_ref.parse::<u32>().is_ok();

    let prompt = if is_pr_number {
        // Use gh CLI to get PR info
        format!(
            r#"Please review Pull Request #{}.

Use the following commands to gather information:
1. `gh pr view {}` - Get PR details
2. `gh pr diff {}` - Get the diff
3. `gh pr checks {}` - Check CI status

Then provide a comprehensive code review covering:
- **Summary**: What does this PR do?
- **Code Quality**: Are there any issues with the code?
- **Security**: Any security concerns?
- **Performance**: Any performance implications?
- **Testing**: Is the code adequately tested?
- **Suggestions**: Specific improvements to consider

Please be constructive and specific in your feedback."#,
            pr_ref, pr_ref, pr_ref, pr_ref
        )
    } else {
        // Review branch diff against main
        format!(
            r#"Please review the changes in branch '{}' compared to the main branch.

Use the following commands to gather information:
1. `git log main..{} --oneline` - List commits
2. `git diff main...{}` - Get the diff

Then provide a comprehensive code review covering:
- **Summary**: What do these changes do?
- **Code Quality**: Are there any issues with the code?
- **Security**: Any security concerns?
- **Performance**: Any performance implications?
- **Testing**: Is the code adequately tested?
- **Suggestions**: Specific improvements to consider

Please be constructive and specific in your feedback."#,
            pr_ref, pr_ref, pr_ref
        )
    };

    Ok(CommandResult::prompt(prompt)
        .with_status(format!("Reviewing {}...", pr_ref))
        .with_tool_restrictions(vec![
            "Bash".to_string(),
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
        ]))
}
