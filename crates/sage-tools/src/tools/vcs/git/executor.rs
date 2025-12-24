//! Git command execution

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use tokio::process::Command as TokioCommand;
use tracing::debug;

use super::types::GitTool;

impl GitTool {
    /// Execute a git command
    pub async fn execute_git_command(
        &self,
        args: &[&str],
        working_dir: Option<&str>,
    ) -> Result<String> {
        let mut cmd = TokioCommand::new("git");
        cmd.args(args);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        debug!("Executing git command: git {}", args.join(" "));

        let output = cmd
            .output()
            .await
            .with_context(|| format!("Failed to execute git command: git {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Git command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Check if directory is a git repository
    pub async fn is_git_repo(&self, path: &str) -> Result<bool> {
        let git_dir = Path::new(path).join(".git");
        Ok(git_dir.exists())
    }

    /// Get git repository root
    pub async fn get_repo_root(&self, working_dir: Option<&str>) -> Result<String> {
        let output = self
            .execute_git_command(&["rev-parse", "--show-toplevel"], working_dir)
            .await?;
        Ok(output.trim().to_string())
    }
}
