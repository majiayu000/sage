//! Git Version Control Tool
//!
//! This tool provides comprehensive Git operations including:
//! - Branch management
//! - Commit operations
//! - Status and diff operations
//! - Remote operations
//! - Merge and conflict resolution
//! - Repository information

mod executor;
mod operations;
mod schema;
mod types;

pub use types::{GitOperation, GitTool, GitToolParams};

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use tracing::info;

use sage_core::tools::{Tool, ToolResult};

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        Self::get_parameters_schema()
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: GitToolParams =
            serde_json::from_value(params).context("Failed to parse Git tool parameters")?;

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

            GitOperation::CreateBranch { .. }
            | GitOperation::SwitchBranch { .. }
            | GitOperation::DeleteBranch { .. }
            | GitOperation::ListBranches => {
                self.handle_branch_operation(&params.operation, working_dir)
                    .await?
            }

            GitOperation::Add { .. } | GitOperation::Commit { .. } | GitOperation::Reset { .. } => {
                self.handle_commit_operation(&params.operation, working_dir)
                    .await?
            }

            GitOperation::Push { .. }
            | GitOperation::Pull { .. }
            | GitOperation::Clone { .. }
            | GitOperation::Remote { .. } => {
                self.handle_remote_operation(&params.operation, working_dir)
                    .await?
            }

            GitOperation::Diff { .. }
            | GitOperation::Log { .. }
            | GitOperation::Info
            | GitOperation::Blame { .. }
            | GitOperation::FileHistory { .. } => {
                self.handle_info_operation(&params.operation, working_dir)
                    .await?
            }

            GitOperation::Stash { .. }
            | GitOperation::ListStashes
            | GitOperation::ApplyStash { .. } => {
                self.handle_stash_operation(&params.operation, working_dir)
                    .await?
            }

            GitOperation::Merge { .. } | GitOperation::Rebase { .. } => {
                self.handle_merge_operation(&params.operation, working_dir)
                    .await?
            }
        };

        Ok(ToolResult::new(result, HashMap::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

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
            .args(["init"])
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
