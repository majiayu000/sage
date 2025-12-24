//! Git merge operations

use anyhow::{anyhow, Result};

use crate::tools::vcs::git::types::{GitOperation, GitTool};

impl GitTool {
    /// Handle merge operations
    pub async fn handle_merge_operation(
        &self,
        operation: &GitOperation,
        working_dir: Option<&str>,
    ) -> Result<String> {
        match operation {
            GitOperation::Merge { branch } => {
                self.execute_git_command(&["merge", branch], working_dir)
                    .await?;
                Ok(format!("Merged branch '{}'", branch))
            }
            GitOperation::Rebase { branch } => {
                self.execute_git_command(&["rebase", branch], working_dir)
                    .await?;
                Ok(format!("Rebased onto '{}'", branch))
            }
            _ => Err(anyhow!("Invalid merge operation")),
        }
    }
}
