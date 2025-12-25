//! Git repository information extraction

use std::path::Path;

use super::models::GitInfo;

/// Get git repository information
pub fn get_git_info(root: &Path) -> Option<GitInfo> {
    let git_dir = root.join(".git");
    if !git_dir.exists() {
        return None;
    }

    let mut info = GitInfo {
        is_repo: true,
        branch: None,
        remote_url: None,
        has_changes: false,
        commit_count: None,
    };

    // Get current branch
    let head = git_dir.join("HEAD");
    if let Ok(content) = std::fs::read_to_string(head) {
        if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
            info.branch = Some(branch.trim().to_string());
        }
    }

    // Get remote URL
    let config = git_dir.join("config");
    if let Ok(content) = std::fs::read_to_string(config) {
        for line in content.lines() {
            let line = line.trim();
            if let Some(url) = line.strip_prefix("url = ") {
                info.remote_url = Some(url.to_string());
                break;
            }
        }
    }

    Some(info)
}
