//! Context builder for loading project context
//!
//! This module provides functionality to load CLAUDE.md and project-specific
//! instructions that should be included in the system prompt.

use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Git repository information for context
#[derive(Debug, Clone, Default)]
pub struct GitInfo {
    /// Whether this is a git repository
    pub is_repo: bool,
    /// Current branch name
    pub branch: Option<String>,
    /// Main/default branch name
    pub main_branch: Option<String>,
    /// Git status output (for system prompt)
    pub status: Option<String>,
}

impl GitInfo {
    /// Detect git info from the working directory
    pub fn detect(working_dir: &Path) -> Self {
        let mut info = Self::default();

        // Check if directory is a git repo
        let git_dir = working_dir.join(".git");
        info.is_repo = git_dir.exists();

        if !info.is_repo {
            return info;
        }

        // Detect current branch
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(working_dir)
            .output()
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() {
                    info.branch = Some(branch);
                }
            }
        }

        // Detect main branch
        info.main_branch = Self::detect_main_branch(working_dir);

        // Get git status (short format)
        if let Ok(output) = std::process::Command::new("git")
            .args(["status", "--short"])
            .current_dir(working_dir)
            .output()
        {
            if output.status.success() {
                let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !status.is_empty() {
                    info.status = Some(status);
                }
            }
        }

        info
    }

    fn detect_main_branch(working_dir: &Path) -> Option<String> {
        // Try to get the default branch from remote
        if let Ok(output) = std::process::Command::new("git")
            .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
            .current_dir(working_dir)
            .output()
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Some(name) = branch.strip_prefix("origin/") {
                    return Some(name.to_string());
                }
            }
        }
        // Fallback to main or master
        for branch in &["main", "master"] {
            if let Ok(output) = std::process::Command::new("git")
                .args(["rev-parse", "--verify", branch])
                .current_dir(working_dir)
                .output()
            {
                if output.status.success() {
                    return Some(branch.to_string());
                }
            }
        }
        Some("main".to_string())
    }
}

/// Project context loaded from files
#[derive(Debug, Clone, Default)]
pub struct ProjectContext {
    /// Contents of CLAUDE.md (if exists)
    pub claude_md: Option<String>,
    /// Contents of .sage/instructions.md (if exists)
    pub project_instructions: Option<String>,
    /// Git repository information
    pub git_info: Option<GitInfo>,
}

/// Builder for loading project context
#[derive(Debug)]
pub struct ContextBuilder {
    working_dir: PathBuf,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
        }
    }

    /// Load CLAUDE.md file (if exists)
    pub fn load_claude_md(&self) -> Option<String> {
        let path = self.working_dir.join("CLAUDE.md");
        Self::read_file_if_exists(&path, "CLAUDE.md")
    }

    /// Load .sage/instructions.md (if exists)
    pub fn load_project_instructions(&self) -> Option<String> {
        let path = self.working_dir.join(".sage/instructions.md");
        Self::read_file_if_exists(&path, ".sage/instructions.md")
    }

    /// Load git information
    pub fn load_git_info(&self) -> Option<GitInfo> {
        let info = GitInfo::detect(&self.working_dir);
        if info.is_repo { Some(info) } else { None }
    }

    /// Build complete project context
    pub fn build_context(&self) -> ProjectContext {
        let claude_md = self.load_claude_md();
        let project_instructions = self.load_project_instructions();
        let git_info = self.load_git_info();

        if claude_md.is_some() || project_instructions.is_some() {
            info!(
                "Loaded project context: claude_md={}, instructions={}",
                claude_md.is_some(),
                project_instructions.is_some()
            );
        }

        ProjectContext {
            claude_md,
            project_instructions,
            git_info,
        }
    }

    /// Read a file if it exists, logging the result
    fn read_file_if_exists(path: &Path, name: &str) -> Option<String> {
        if !path.exists() {
            debug!("{} not found at {:?}", name, path);
            return None;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    debug!("{} exists but is empty", name);
                    None
                } else {
                    info!("Loaded {} ({} bytes)", name, trimmed.len());
                    Some(trimmed.to_string())
                }
            }
            Err(e) => {
                warn!("Failed to read {}: {}", name, e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_context_builder_no_files() {
        let dir = tempdir().unwrap();
        let builder = ContextBuilder::new(dir.path());
        let ctx = builder.build_context();

        assert!(ctx.claude_md.is_none());
        assert!(ctx.project_instructions.is_none());
    }

    #[test]
    fn test_load_claude_md() {
        let dir = tempdir().unwrap();
        let content = "# Test CLAUDE.md\nSome instructions.";
        fs::write(dir.path().join("CLAUDE.md"), content).unwrap();

        let builder = ContextBuilder::new(dir.path());
        let claude_md = builder.load_claude_md();

        assert!(claude_md.is_some());
        assert_eq!(claude_md.unwrap(), content);
    }

    #[test]
    fn test_load_project_instructions() {
        let dir = tempdir().unwrap();
        let sage_dir = dir.path().join(".sage");
        fs::create_dir_all(&sage_dir).unwrap();
        let content = "# Project Instructions";
        fs::write(sage_dir.join("instructions.md"), content).unwrap();

        let builder = ContextBuilder::new(dir.path());
        let instructions = builder.load_project_instructions();

        assert!(instructions.is_some());
        assert_eq!(instructions.unwrap(), content);
    }

    #[test]
    fn test_empty_file_returns_none() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "   \n  ").unwrap();

        let builder = ContextBuilder::new(dir.path());
        assert!(builder.load_claude_md().is_none());
    }
}
