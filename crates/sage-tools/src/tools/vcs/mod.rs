//! Version Control System Tools
//!
//! This module provides tools for interacting with version control systems,
//! particularly Git.

pub mod git_simple;

pub use git_simple::GitTool;

use std::sync::Arc;
use sage_core::tools::Tool;

/// Get all version control tools
pub fn get_vcs_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(GitTool::new()),
    ]
}