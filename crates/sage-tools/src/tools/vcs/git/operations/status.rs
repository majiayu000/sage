//! Git status operation

use anyhow::Result;

use crate::tools::vcs::git::types::GitTool;

impl GitTool {
    /// Handle status operation
    pub async fn handle_status(&self, working_dir: Option<&str>) -> Result<String> {
        let output = self
            .execute_git_command(&["status", "--porcelain"], working_dir)
            .await?;

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
}
