//! Git commit operations

use anyhow::{anyhow, Result};

use crate::tools::vcs::git::types::{GitOperation, GitTool};

impl GitTool {
    /// Handle commit operations
    pub async fn handle_commit_operation(
        &self,
        operation: &GitOperation,
        working_dir: Option<&str>,
    ) -> Result<String> {
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
}
