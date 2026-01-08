//! Working directory configuration for sub-agents
//!
//! Defines how sub-agents inherit or configure their working directory.
//! This is critical for file operations and relative path resolution.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Working directory configuration for sub-agents
///
/// Controls how a sub-agent determines its working directory during execution.
/// This affects all file-related operations like Read, Write, Edit, and Bash commands.
///
/// # Examples
///
/// ```rust
/// use sage_core::agent::subagent::WorkingDirectoryConfig;
/// use std::path::PathBuf;
///
/// // Inherit from parent (default behavior for sub-agents)
/// let inherited = WorkingDirectoryConfig::Inherited;
///
/// // Use explicit path
/// let explicit = WorkingDirectoryConfig::Explicit(PathBuf::from("/path/to/project"));
///
/// // Use current process working directory
/// let process_cwd = WorkingDirectoryConfig::ProcessCwd;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkingDirectoryConfig {
    /// Inherit working directory from parent agent
    ///
    /// This is the recommended default for sub-agents. It ensures that
    /// file operations work relative to the same directory as the parent.
    Inherited,

    /// Use an explicitly specified working directory
    ///
    /// Useful when the sub-agent needs to operate in a different directory
    /// than the parent, such as when analyzing a different project.
    Explicit(PathBuf),

    /// Use the current process working directory
    ///
    /// Falls back to `std::env::current_dir()`. This was the old behavior
    /// before inheritance was implemented. Generally not recommended for
    /// sub-agents as it may differ from parent's working directory.
    ProcessCwd,
}

impl Default for WorkingDirectoryConfig {
    fn default() -> Self {
        // Default to inherited - this is the safest option for sub-agents
        WorkingDirectoryConfig::Inherited
    }
}

impl fmt::Display for WorkingDirectoryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkingDirectoryConfig::Inherited => write!(f, "inherited"),
            WorkingDirectoryConfig::Explicit(path) => write!(f, "explicit({})", path.display()),
            WorkingDirectoryConfig::ProcessCwd => write!(f, "process_cwd"),
        }
    }
}

impl WorkingDirectoryConfig {
    /// Resolve the actual working directory path
    ///
    /// # Arguments
    /// * `parent_cwd` - The parent agent's working directory (used for Inherited)
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The resolved absolute working directory path
    /// * `Err(std::io::Error)` - If directory resolution fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sage_core::agent::subagent::WorkingDirectoryConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = WorkingDirectoryConfig::Inherited;
    /// let parent_cwd = PathBuf::from("/parent/project");
    /// let resolved = config.resolve(Some(&parent_cwd)).unwrap();
    /// assert_eq!(resolved, PathBuf::from("/parent/project"));
    /// ```
    pub fn resolve(&self, parent_cwd: Option<&PathBuf>) -> std::io::Result<PathBuf> {
        match self {
            WorkingDirectoryConfig::Inherited => {
                // If parent_cwd is provided, use it; otherwise fall back to process cwd
                match parent_cwd {
                    Some(cwd) => Ok(cwd.clone()),
                    None => {
                        tracing::warn!(
                            "WorkingDirectoryConfig::Inherited used but no parent_cwd provided, \
                             falling back to process cwd"
                        );
                        std::env::current_dir()
                    }
                }
            }
            WorkingDirectoryConfig::Explicit(path) => {
                // Canonicalize the explicit path
                if path.is_absolute() {
                    Ok(path.clone())
                } else {
                    // Resolve relative path against process cwd
                    let cwd = std::env::current_dir()?;
                    Ok(cwd.join(path))
                }
            }
            WorkingDirectoryConfig::ProcessCwd => std::env::current_dir(),
        }
    }

    /// Check if this configuration requires a parent working directory
    pub fn requires_parent_cwd(&self) -> bool {
        matches!(self, WorkingDirectoryConfig::Inherited)
    }

    /// Create an explicit configuration from a path
    pub fn explicit(path: impl Into<PathBuf>) -> Self {
        WorkingDirectoryConfig::Explicit(path.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_is_inherited() {
        assert_eq!(WorkingDirectoryConfig::default(), WorkingDirectoryConfig::Inherited);
    }

    #[test]
    fn test_display() {
        assert_eq!(WorkingDirectoryConfig::Inherited.to_string(), "inherited");
        assert_eq!(WorkingDirectoryConfig::ProcessCwd.to_string(), "process_cwd");
        assert_eq!(
            WorkingDirectoryConfig::Explicit(PathBuf::from("/foo/bar")).to_string(),
            "explicit(/foo/bar)"
        );
    }

    #[test]
    fn test_resolve_inherited_with_parent() {
        let config = WorkingDirectoryConfig::Inherited;
        let parent = PathBuf::from("/parent/dir");
        let resolved = config.resolve(Some(&parent)).unwrap();
        assert_eq!(resolved, PathBuf::from("/parent/dir"));
    }

    #[test]
    fn test_resolve_inherited_without_parent() {
        let config = WorkingDirectoryConfig::Inherited;
        let resolved = config.resolve(None).unwrap();
        // Should fall back to current directory
        assert_eq!(resolved, env::current_dir().unwrap());
    }

    #[test]
    fn test_resolve_explicit() {
        let config = WorkingDirectoryConfig::Explicit(PathBuf::from("/explicit/path"));
        let resolved = config.resolve(None).unwrap();
        assert_eq!(resolved, PathBuf::from("/explicit/path"));
    }

    #[test]
    fn test_resolve_process_cwd() {
        let config = WorkingDirectoryConfig::ProcessCwd;
        let resolved = config.resolve(None).unwrap();
        assert_eq!(resolved, env::current_dir().unwrap());
    }

    #[test]
    fn test_requires_parent_cwd() {
        assert!(WorkingDirectoryConfig::Inherited.requires_parent_cwd());
        assert!(!WorkingDirectoryConfig::ProcessCwd.requires_parent_cwd());
        assert!(!WorkingDirectoryConfig::Explicit(PathBuf::from("/foo")).requires_parent_cwd());
    }

    #[test]
    fn test_serde_roundtrip() {
        let configs = vec![
            WorkingDirectoryConfig::Inherited,
            WorkingDirectoryConfig::ProcessCwd,
            WorkingDirectoryConfig::Explicit(PathBuf::from("/test/path")),
        ];

        for config in configs {
            let json = serde_json::to_string(&config).unwrap();
            let deserialized: WorkingDirectoryConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(config, deserialized);
        }
    }
}
