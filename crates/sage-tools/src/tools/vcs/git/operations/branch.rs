//! Git branch operations

use anyhow::{anyhow, Result};

use crate::tools::vcs::git::types::{GitOperation, GitTool};

impl GitTool {
    /// Handle branch operations
    pub async fn handle_branch_operation(
        &self,
        operation: &GitOperation,
        working_dir: Option<&str>,
    ) -> Result<String> {
        match operation {
            GitOperation::CreateBranch { name } => {
                self.execute_git_command(&["checkout", "-b", name], working_dir)
                    .await?;
                Ok(format!("Created and switched to branch '{}'", name))
            }
            GitOperation::SwitchBranch { name } => {
                self.execute_git_command(&["checkout", name], working_dir)
                    .await?;
                Ok(format!("Switched to branch '{}'", name))
            }
            GitOperation::DeleteBranch { name, force } => {
                let flag = if *force { "-D" } else { "-d" };
                self.execute_git_command(&["branch", flag, name], working_dir)
                    .await?;
                Ok(format!("Deleted branch '{}'", name))
            }
            GitOperation::ListBranches => {
                let output = self
                    .execute_git_command(&["branch", "-a"], working_dir)
                    .await?;
                Ok(format!("Branches:\n{}", output))
            }
            _ => Err(anyhow!("Invalid branch operation")),
        }
    }
}
