//! Git remote operations

use anyhow::{anyhow, Result};

use crate::tools::vcs::git::types::{GitOperation, GitTool};

impl GitTool {
    /// Handle remote operations
    pub async fn handle_remote_operation(
        &self,
        operation: &GitOperation,
        working_dir: Option<&str>,
    ) -> Result<String> {
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
}
