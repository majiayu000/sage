//! Git stash operations

use anyhow::{anyhow, Result};

use crate::tools::vcs::git::types::{GitOperation, GitTool};

impl GitTool {
    /// Handle stash operations
    pub async fn handle_stash_operation(
        &self,
        operation: &GitOperation,
        working_dir: Option<&str>,
    ) -> Result<String> {
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
                let output = self
                    .execute_git_command(&["stash", "list"], working_dir)
                    .await?;
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
}
