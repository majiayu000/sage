//! Git tool type definitions

use serde::{Deserialize, Serialize};

/// Git operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOperation {
    /// Get repository status
    Status,
    /// Create a new branch
    CreateBranch { name: String },
    /// Switch to a branch
    SwitchBranch { name: String },
    /// Delete a branch
    DeleteBranch { name: String, force: bool },
    /// List branches
    ListBranches,
    /// Add files to staging
    Add { files: Vec<String> },
    /// Commit changes
    Commit { message: String, all: bool },
    /// Push changes
    Push {
        remote: Option<String>,
        branch: Option<String>,
    },
    /// Pull changes
    Pull {
        remote: Option<String>,
        branch: Option<String>,
    },
    /// Show diff
    Diff { staged: bool, file: Option<String> },
    /// Show log
    Log { count: Option<usize>, oneline: bool },
    /// Clone repository
    Clone { url: String, path: Option<String> },
    /// Reset changes
    Reset { hard: bool, commit: Option<String> },
    /// Show remote information
    Remote { verbose: bool },
    /// Merge branch
    Merge { branch: String },
    /// Rebase branch
    Rebase { branch: String },
    /// Show repository info
    Info,
    /// Stash changes
    Stash { message: Option<String> },
    /// List stashes
    ListStashes,
    /// Apply stash
    ApplyStash { index: Option<usize> },
    /// Show blame
    Blame { file: String },
    /// Show file history
    FileHistory { file: String },
}

/// Git tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitToolParams {
    /// Git operation to perform
    pub operation: GitOperation,
    /// Working directory (optional, defaults to current directory)
    pub working_dir: Option<String>,
}

/// Git tool for version control operations
#[derive(Debug, Clone)]
pub struct GitTool {
    pub(crate) name: String,
    pub(crate) description: String,
}

impl GitTool {
    /// Create a new Git tool
    pub fn new() -> Self {
        Self {
            name: "git".to_string(),
            description: "Git version control operations including branch management, commits, and repository information".to_string(),
        }
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}
