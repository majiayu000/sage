//! Git Version Control Tool
//!
//! This tool provides comprehensive Git operations including:
//! - Branch management
//! - Commit operations
//! - Status and diff operations
//! - Remote operations
//! - Merge and conflict resolution
//! - Repository information

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::env;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context, anyhow};
use tokio::process::Command as TokioCommand;
use tracing::{info, debug, error};

use sage_core::tools::{Tool, ToolResult};

/// Git operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOperation {
    /// Get repository status
    Status,
    /// Create a new branch
    CreateBranch { name: String },
    /// Switch to a branch
    SwitchBranch { name: String },
    /// Delete a branch
    DeleteBranch { name: String, force: bool },
    /// List branches
    ListBranches,
    /// Add files to staging
    Add { files: Vec<String> },
    /// Commit changes
    Commit { message: String, all: bool },
    /// Push changes
    Push { remote: Option<String>, branch: Option<String> },
    /// Pull changes
    Pull { remote: Option<String>, branch: Option<String> },
    /// Show diff
    Diff { staged: bool, file: Option<String> },
    /// Show log
    Log { count: Option<usize>, oneline: bool },
    /// Clone repository
    Clone { url: String, path: Option<String> },
    /// Reset changes
    Reset { hard: bool, commit: Option<String> },
    /// Show remote information
    Remote { verbose: bool },
    /// Merge branch
    Merge { branch: String },
    /// Rebase branch
    Rebase { branch: String },
    /// Show repository info
    Info,
    /// Stash changes
    Stash { message: Option<String> },
    /// List stashes
    ListStashes,
    /// Apply stash
    ApplyStash { index: Option<usize> },
    /// Show blame
    Blame { file: String },
    /// Show file history
    FileHistory { file: String },
}

/// Git tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitToolParams {
    /// Git operation to perform
    pub operation: GitOperation,
    /// Working directory (optional, defaults to current directory)
    pub working_dir: Option<String>,
}

/// Git tool for version control operations
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
            description: "Git version control operations including branch management, commits, and repository information".to_string(),
        }
    }

    /// Execute a git command
    async fn execute_git_command(&self, args: &[&str], working_dir: Option<&str>) -> Result<String> {
        let mut cmd = TokioCommand::new("git");
        cmd.args(args);
        
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        debug!("Executing git command: git {}", args.join(" "));
        
        let output = cmd.output().await
            .with_context(|| format!("Failed to execute git command: git {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Git command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Check if directory is a git repository
    async fn is_git_repo(&self, path: &str) -> Result<bool> {
        let git_dir = Path::new(path).join(".git");
        Ok(git_dir.exists())
    }

    /// Get git repository root
    async fn get_repo_root(&self, working_dir: Option<&str>) -> Result<String> {
        let output = self.execute_git_command(&["rev-parse", "--show-toplevel"], working_dir).await?;
        Ok(output.trim().to_string())
    }

    /// Handle status operation
    async fn handle_status(&self, working_dir: Option<&str>) -> Result<String> {
        let output = self.execute_git_command(&["status", "--porcelain"], working_dir).await?;
        
        if output.trim().is_empty() {
            return Ok("Working tree clean".to_string());
        }

        let mut result = String::new();
        result.push_str("Repository status:\n");
        
        for line in output.lines() {
            if line.len() >= 3 {
                let status = &line[0..2];
                let file = &line[3..];
                
                let status_desc = match status {
                    "??" => "Untracked",
                    "M " => "Modified",
                    " M" => "Modified (not staged)",
                    "A " => "Added",
                    " A" => "Added (not staged)",
                    "D " => "Deleted",
                    " D" => "Deleted (not staged)",
                    "R " => "Renamed",
                    "C " => "Copied",
                    "MM" => "Modified (staged and unstaged)",
                    _ => "Unknown",
                };
                
                result.push_str(&format!("  {}: {}\n", status_desc, file));
            }
        }
        
        Ok(result)
    }

    /// Handle branch operations
    async fn handle_branch_operation(&self, operation: &GitOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            GitOperation::CreateBranch { name } => {
                self.execute_git_command(&["checkout", "-b", name], working_dir).await?;
                Ok(format!("Created and switched to branch '{}'", name))
            }
            GitOperation::SwitchBranch { name } => {
                self.execute_git_command(&["checkout", name], working_dir).await?;
                Ok(format!("Switched to branch '{}'", name))
            }
            GitOperation::DeleteBranch { name, force } => {
                let flag = if *force { "-D" } else { "-d" };
                self.execute_git_command(&["branch", flag, name], working_dir).await?;
                Ok(format!("Deleted branch '{}'", name))
            }
            GitOperation::ListBranches => {
                let output = self.execute_git_command(&["branch", "-a"], working_dir).await?;
                Ok(format!("Branches:\n{}", output))
            }
            _ => Err(anyhow!("Invalid branch operation")),
        }
    }

    /// Handle commit operations
    async fn handle_commit_operation(&self, operation: &GitOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            GitOperation::Add { files } => {
                let mut args = vec!["add"];
                let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
                args.extend(file_refs);
                self.execute_git_command(&args, working_dir).await?;
                Ok(format!("Added files: {}", files.join(", ")))
            }
            GitOperation::Commit { message, all } => {
                let mut args = vec!["commit", "-m", message];
                if *all {
                    args.insert(1, "-a");
                }
                self.execute_git_command(&args, working_dir).await?;
                Ok(format!("Committed changes: {}", message))
            }
            GitOperation::Reset { hard, commit } => {
                let mut args = vec!["reset"];
                if *hard {
                    args.push("--hard");
                }
                if let Some(commit) = commit {
                    args.push(commit);
                }
                self.execute_git_command(&args, working_dir).await?;
                Ok("Reset completed".to_string())
            }
            _ => Err(anyhow!("Invalid commit operation")),
        }
    }

    /// Handle remote operations
    async fn handle_remote_operation(&self, operation: &GitOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            GitOperation::Push { remote, branch } => {
                let mut args = vec!["push"];
                if let Some(remote) = remote {
                    args.push(remote);
                }
                if let Some(branch) = branch {
                    args.push(branch);
                }
                self.execute_git_command(&args, working_dir).await?;
                Ok("Push completed".to_string())
            }
            GitOperation::Pull { remote, branch } => {
                let mut args = vec!["pull"];
                if let Some(remote) = remote {
                    args.push(remote);
                }
                if let Some(branch) = branch {
                    args.push(branch);
                }
                self.execute_git_command(&args, working_dir).await?;
                Ok("Pull completed".to_string())
            }
            GitOperation::Clone { url, path } => {
                let mut args = vec!["clone", url];
                if let Some(path) = path {
                    args.push(path);
                }
                self.execute_git_command(&args, working_dir).await?;
                Ok("Clone completed".to_string())
            }
            GitOperation::Remote { verbose } => {
                let mut args = vec!["remote"];
                if *verbose {
                    args.push("-v");
                }
                let output = self.execute_git_command(&args, working_dir).await?;
                Ok(format!("Remotes:\n{}", output))
            }
            _ => Err(anyhow!("Invalid remote operation")),
        }
    }

    /// Handle diff and log operations
    async fn handle_info_operation(&self, operation: &GitOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            GitOperation::Diff { staged, file } => {
                let mut args = vec!["diff"];
                if *staged {
                    args.push("--cached");
                }
                if let Some(file) = file {
                    args.push(file);
                }
                let output = self.execute_git_command(&args, working_dir).await?;
                Ok(output)
            }
            GitOperation::Log { count, oneline } => {
                let mut args = vec!["log"];
                if *oneline {
                    args.push("--oneline");
                }
                if let Some(count) = count {
                    args.push("-n");
                    args.push(&count.to_string());
                }
                let output = self.execute_git_command(&args, working_dir).await?;
                Ok(output)
            }
            GitOperation::Info => {
                let mut result = String::new();
                
                // Repository root
                if let Ok(root) = self.get_repo_root(working_dir).await {
                    result.push_str(&format!("Repository root: {}\n", root));
                }
                
                // Current branch
                if let Ok(branch) = self.execute_git_command(&["branch", "--show-current"], working_dir).await {
                    result.push_str(&format!("Current branch: {}\n", branch.trim()));
                }
                
                // Remote URL
                if let Ok(remote) = self.execute_git_command(&["config", "--get", "remote.origin.url"], working_dir).await {
                    result.push_str(&format!("Remote URL: {}\n", remote.trim()));
                }
                
                // Last commit
                if let Ok(commit) = self.execute_git_command(&["log", "-1", "--oneline"], working_dir).await {
                    result.push_str(&format!("Last commit: {}\n", commit.trim()));
                }
                
                Ok(result)
            }
            GitOperation::Blame { file } => {
                let output = self.execute_git_command(&["blame", file], working_dir).await?;
                Ok(output)
            }
            GitOperation::FileHistory { file } => {
                let output = self.execute_git_command(&["log", "--follow", "--oneline", file], working_dir).await?;
                Ok(format!("File history for {}:\n{}", file, output))
            }
            _ => Err(anyhow!("Invalid info operation")),
        }
    }

    /// Handle stash operations
    async fn handle_stash_operation(&self, operation: &GitOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            GitOperation::Stash { message } => {
                let mut args = vec!["stash"];
                if let Some(msg) = message {
                    args.extend(vec!["push", "-m", msg]);
                }
                self.execute_git_command(&args, working_dir).await?;
                Ok("Changes stashed".to_string())
            }
            GitOperation::ListStashes => {
                let output = self.execute_git_command(&["stash", "list"], working_dir).await?;
                Ok(format!("Stashes:\n{}", output))
            }
            GitOperation::ApplyStash { index } => {
                let mut args = vec!["stash", "apply"];
                if let Some(idx) = index {
                    args.push(&format!("stash@{{{}}}", idx));
                }
                self.execute_git_command(&args, working_dir).await?;
                Ok("Stash applied".to_string())
            }
            _ => Err(anyhow!("Invalid stash operation")),
        }
    }

    /// Handle merge operations
    async fn handle_merge_operation(&self, operation: &GitOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            GitOperation::Merge { branch } => {
                self.execute_git_command(&["merge", branch], working_dir).await?;
                Ok(format!("Merged branch '{}'", branch))
            }
            GitOperation::Rebase { branch } => {
                self.execute_git_command(&["rebase", branch], working_dir).await?;
                Ok(format!("Rebased onto '{}'", branch))
            }
            _ => Err(anyhow!("Invalid merge operation")),
        }
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

    fn parameters_json_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "object",
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "status": { "type": "null" }
                            },
                            "required": ["status"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "create_branch": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["create_branch"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "switch_branch": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["switch_branch"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "delete_branch": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" },
                                        "force": { "type": "boolean", "default": false }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["delete_branch"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "list_branches": { "type": "null" }
                            },
                            "required": ["list_branches"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "add": {
                                    "type": "object",
                                    "properties": {
                                        "files": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        }
                                    },
                                    "required": ["files"]
                                }
                            },
                            "required": ["add"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "commit": {
                                    "type": "object",
                                    "properties": {
                                        "message": { "type": "string" },
                                        "all": { "type": "boolean", "default": false }
                                    },
                                    "required": ["message"]
                                }
                            },
                            "required": ["commit"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "push": {
                                    "type": "object",
                                    "properties": {
                                        "remote": { "type": "string" },
                                        "branch": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["push"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "pull": {
                                    "type": "object",
                                    "properties": {
                                        "remote": { "type": "string" },
                                        "branch": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["pull"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "diff": {
                                    "type": "object",
                                    "properties": {
                                        "staged": { "type": "boolean", "default": false },
                                        "file": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["diff"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "log": {
                                    "type": "object",
                                    "properties": {
                                        "count": { "type": "integer", "minimum": 1 },
                                        "oneline": { "type": "boolean", "default": false }
                                    }
                                }
                            },
                            "required": ["log"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "clone": {
                                    "type": "object",
                                    "properties": {
                                        "url": { "type": "string" },
                                        "path": { "type": "string" }
                                    },
                                    "required": ["url"]
                                }
                            },
                            "required": ["clone"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "reset": {
                                    "type": "object",
                                    "properties": {
                                        "hard": { "type": "boolean", "default": false },
                                        "commit": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["reset"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "remote": {
                                    "type": "object",
                                    "properties": {
                                        "verbose": { "type": "boolean", "default": false }
                                    }
                                }
                            },
                            "required": ["remote"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "merge": {
                                    "type": "object",
                                    "properties": {
                                        "branch": { "type": "string" }
                                    },
                                    "required": ["branch"]
                                }
                            },
                            "required": ["merge"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "rebase": {
                                    "type": "object",
                                    "properties": {
                                        "branch": { "type": "string" }
                                    },
                                    "required": ["branch"]
                                }
                            },
                            "required": ["rebase"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "info": { "type": "null" }
                            },
                            "required": ["info"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "stash": {
                                    "type": "object",
                                    "properties": {
                                        "message": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["stash"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "list_stashes": { "type": "null" }
                            },
                            "required": ["list_stashes"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "apply_stash": {
                                    "type": "object",
                                    "properties": {
                                        "index": { "type": "integer", "minimum": 0 }
                                    }
                                }
                            },
                            "required": ["apply_stash"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "blame": {
                                    "type": "object",
                                    "properties": {
                                        "file": { "type": "string" }
                                    },
                                    "required": ["file"]
                                }
                            },
                            "required": ["blame"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "file_history": {
                                    "type": "object",
                                    "properties": {
                                        "file": { "type": "string" }
                                    },
                                    "required": ["file"]
                                }
                            },
                            "required": ["file_history"],
                            "additionalProperties": false
                        }
                    ],
                    "description": "Git operation to perform"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory (optional, defaults to current directory)"
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: GitToolParams = serde_json::from_value(params)
            .context("Failed to parse Git tool parameters")?;

        let working_dir = params.working_dir.as_deref();
        
        // Validate working directory if provided
        if let Some(dir) = working_dir {
            if !Path::new(dir).exists() {
                return Err(anyhow!("Working directory does not exist: {}", dir));
            }
        }

        info!("Executing Git operation: {:?}", params.operation);

        let result = match &params.operation {
            GitOperation::Status => self.handle_status(working_dir).await?,
            
            GitOperation::CreateBranch { .. } |
            GitOperation::SwitchBranch { .. } |
            GitOperation::DeleteBranch { .. } |
            GitOperation::ListBranches => self.handle_branch_operation(&params.operation, working_dir).await?,
            
            GitOperation::Add { .. } |
            GitOperation::Commit { .. } |
            GitOperation::Reset { .. } => self.handle_commit_operation(&params.operation, working_dir).await?,
            
            GitOperation::Push { .. } |
            GitOperation::Pull { .. } |
            GitOperation::Clone { .. } |
            GitOperation::Remote { .. } => self.handle_remote_operation(&params.operation, working_dir).await?,
            
            GitOperation::Diff { .. } |
            GitOperation::Log { .. } |
            GitOperation::Info |
            GitOperation::Blame { .. } |
            GitOperation::FileHistory { .. } => self.handle_info_operation(&params.operation, working_dir).await?,
            
            GitOperation::Stash { .. } |
            GitOperation::ListStashes |
            GitOperation::ApplyStash { .. } => self.handle_stash_operation(&params.operation, working_dir).await?,
            
            GitOperation::Merge { .. } |
            GitOperation::Rebase { .. } => self.handle_merge_operation(&params.operation, working_dir).await?,
        };

        Ok(ToolResult::new(result, HashMap::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_git_tool_creation() {
        let tool = GitTool::new();
        assert_eq!(tool.name(), "git");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_git_status_empty_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        
        // Initialize empty git repo
        Command::new("git")
            .args(&["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        
        let tool = GitTool::new();
        let result = tool.handle_status(Some(repo_path)).await.unwrap();
        assert_eq!(result, "Working tree clean");
    }

    #[tokio::test]
    async fn test_git_tool_schema() {
        let tool = GitTool::new();
        let schema = tool.parameters_json_schema();
        
        assert!(schema.is_object());
        assert!(schema["properties"]["operation"].is_object());
        assert!(schema["properties"]["working_dir"].is_object());
    }
}