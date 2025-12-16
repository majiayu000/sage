//! Simplified Git Tool
//!
//! This tool provides essential Git operations with the new Tool trait interface.

use async_trait::async_trait;
use tokio::process::Command;
use tracing::{debug, info};

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Simple Git tool for version control operations
#[derive(Debug, Clone)]
pub struct GitTool {
    name: String,
    description: String,
}

impl GitTool {
    /// Create a new Git tool
    pub fn new() -> Self {
        Self {
            name: "git".to_string(),
            description: "Git version control operations including status, add, commit, push, pull, and branch management".to_string(),
        }
    }

    /// Execute a git command
    async fn execute_git_command(
        &self,
        args: &[&str],
        working_dir: Option<&str>,
    ) -> Result<String, ToolError> {
        let mut cmd = Command::new("git");
        cmd.args(args);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        debug!("Executing git command: git {}", args.join(" "));

        let output = cmd.output().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to execute git command: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!(
                "Git command failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "command",
                    "Git command to execute (status, add, commit, push, pull, log, branch, etc.)",
                ),
                ToolParameter::optional_string("path", "Working directory path"),
                ToolParameter::optional_string(
                    "message",
                    "Commit message (required for commit command)",
                ),
                ToolParameter::optional_string("branch", "Branch name"),
                ToolParameter::optional_string("remote", "Remote name (default: origin)"),
                ToolParameter::optional_string(
                    "files",
                    "Files to add (space-separated, default: .)",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        let working_dir = call.get_string("path");

        info!("Executing Git command: {}", command);

        let result = match command.as_str() {
            "status" => {
                self.execute_git_command(&["status", "--porcelain"], working_dir.as_deref())
                    .await?
            }
            "add" => {
                let files = call.get_string("files").unwrap_or_else(|| ".".to_string());
                let file_list: Vec<&str> = files.split_whitespace().collect();
                let mut args = vec!["add"];
                args.extend(file_list);
                self.execute_git_command(&args, working_dir.as_deref())
                    .await?
            }
            "commit" => {
                let message = call.get_string("message").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'message' parameter for commit".to_string(),
                    )
                })?;
                self.execute_git_command(&["commit", "-m", &message], working_dir.as_deref())
                    .await?
            }
            "push" => {
                let remote = call
                    .get_string("remote")
                    .unwrap_or_else(|| "origin".to_string());
                let branch = call.get_string("branch");
                let mut args = vec!["push", &remote];
                if let Some(ref branch) = branch {
                    args.push(branch);
                }
                self.execute_git_command(&args, working_dir.as_deref())
                    .await?
            }
            "pull" => {
                self.execute_git_command(&["pull"], working_dir.as_deref())
                    .await?
            }
            "log" => {
                self.execute_git_command(&["log", "--oneline", "-10"], working_dir.as_deref())
                    .await?
            }
            "branch" => {
                let branch_name = call.get_string("branch");
                if let Some(name) = branch_name {
                    // Create new branch
                    self.execute_git_command(&["checkout", "-b", &name], working_dir.as_deref())
                        .await?
                } else {
                    // List branches
                    self.execute_git_command(&["branch"], working_dir.as_deref())
                        .await?
                }
            }
            "checkout" => {
                let branch = call.get_string("branch").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'branch' parameter for checkout".to_string(),
                    )
                })?;
                self.execute_git_command(&["checkout", &branch], working_dir.as_deref())
                    .await?
            }
            "diff" => {
                self.execute_git_command(&["diff"], working_dir.as_deref())
                    .await?
            }
            "remote" => {
                self.execute_git_command(&["remote", "-v"], working_dir.as_deref())
                    .await?
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown git command: {}",
                    command
                )));
            }
        };

        Ok(ToolResult::success(call.id.clone(), self.name(), result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_git_tool_creation() {
        let tool = GitTool::new();
        assert_eq!(tool.name(), "git");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_git_tool_schema() {
        let tool = GitTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "git");
        assert!(!schema.description.is_empty());
    }
}
