//! Git info and diff operations

use anyhow::{anyhow, Result};

use crate::tools::vcs::git::types::{GitOperation, GitTool};

impl GitTool {
    /// Handle diff and log operations
    pub async fn handle_info_operation(
        &self,
        operation: &GitOperation,
        working_dir: Option<&str>,
    ) -> Result<String> {
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
                if let Ok(branch) = self
                    .execute_git_command(&["branch", "--show-current"], working_dir)
                    .await
                {
                    result.push_str(&format!("Current branch: {}\n", branch.trim()));
                }

                // Remote URL
                if let Ok(remote) = self
                    .execute_git_command(&["config", "--get", "remote.origin.url"], working_dir)
                    .await
                {
                    result.push_str(&format!("Remote URL: {}\n", remote.trim()));
                }

                // Last commit
                if let Ok(commit) = self
                    .execute_git_command(&["log", "-1", "--oneline"], working_dir)
                    .await
                {
                    result.push_str(&format!("Last commit: {}\n", commit.trim()));
                }

                Ok(result)
            }
            GitOperation::Blame { file } => {
                let output = self
                    .execute_git_command(&["blame", file], working_dir)
                    .await?;
                Ok(output)
            }
            GitOperation::FileHistory { file } => {
                let output = self
                    .execute_git_command(&["log", "--follow", "--oneline", file], working_dir)
                    .await?;
                Ok(format!("File history for {}:\n{}", file, output))
            }
            _ => Err(anyhow!("Invalid info operation")),
        }
    }
}
