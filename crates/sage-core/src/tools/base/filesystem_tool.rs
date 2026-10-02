//! File system helper trait for tools

use super::ToolError;
use super::tool_trait::Tool;
use std::path::{Component, Path, PathBuf};

/// Helper trait for tools that need access to the file system.
///
/// Provides common functionality for file-based tools including path resolution
/// and security checks to prevent path traversal attacks.
///
/// # Security
///
/// The `is_safe_path()` method prevents malicious paths from escaping the
/// working directory using techniques like `../../../etc/passwd` or symlinks.
///
/// # Examples
///
/// ```no_run
/// use sage_core::tools::{Tool, ToolSchema};
/// use sage_core::tools::base::{FileSystemTool, ToolError};
/// use sage_core::tools::types::{ToolCall, ToolResult};
/// use async_trait::async_trait;
/// use std::path::{Path, PathBuf};
///
/// struct ReadTool {
///     working_dir: PathBuf,
/// }
///
/// #[async_trait]
/// impl Tool for ReadTool {
///     fn name(&self) -> &str { "read" }
///     fn description(&self) -> &str { "Read files" }
///     fn schema(&self) -> ToolSchema {
///         ToolSchema::new(self.name(), self.description(), vec![])
///     }
///
///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
///         let path_str = call.arguments.get("path")
///             .and_then(|v| v.as_str())
///             .ok_or_else(|| ToolError::InvalidArguments("path required".into()))?;
///
///         let path = self.resolve_path(path_str);
///
///         if !self.is_safe_path(&path) {
///             return Err(ToolError::PermissionDenied("Path outside working directory".into()));
///         }
///
///         // Read file...
///         Ok(ToolResult::success(&call.id, self.name(), "file contents"))
///     }
/// }
///
/// impl FileSystemTool for ReadTool {
///     fn working_directory(&self) -> &Path {
///         &self.working_dir
///     }
/// }
/// ```
pub trait FileSystemTool: Tool {
    /// Get the working directory for file operations.
    ///
    /// All file paths should be resolved relative to this directory.
    fn working_directory(&self) -> &Path;

    /// Resolve a relative path to an absolute path.
    ///
    /// If the path is already absolute, it is returned unchanged.
    /// Otherwise, it is joined with the working directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_core::tools::base::FileSystemTool;
    /// # use sage_core::tools::{Tool, ToolSchema};
    /// # use sage_core::tools::base::ToolError;
    /// # use sage_core::tools::types::{ToolCall, ToolResult};
    /// # use async_trait::async_trait;
    /// # use std::path::{Path, PathBuf};
    ///
    /// # struct MyTool { working_dir: PathBuf }
    /// # #[async_trait]
    /// # impl Tool for MyTool {
    /// #     fn name(&self) -> &str { "my_tool" }
    /// #     fn description(&self) -> &str { "A tool" }
    /// #     fn schema(&self) -> ToolSchema { ToolSchema::new(self.name(), self.description(), vec![]) }
    /// #     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    /// #         Ok(ToolResult::success(&call.id, self.name(), "done"))
    /// #     }
    /// # }
    /// # impl FileSystemTool for MyTool {
    /// #     fn working_directory(&self) -> &Path { &self.working_dir }
    /// # }
    ///
    /// # fn example() {
    /// let tool = MyTool { working_dir: PathBuf::from("/home/user/project") };
    ///
    /// // Relative path gets joined with working dir
    /// let resolved = tool.resolve_path("src/main.rs");
    /// assert_eq!(resolved, PathBuf::from("/home/user/project/src/main.rs"));
    ///
    /// // Absolute path is unchanged
    /// let resolved = tool.resolve_path("/etc/hosts");
    /// assert_eq!(resolved, PathBuf::from("/etc/hosts"));
    /// # }
    /// ```
    fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        let path = Path::new(path);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory().join(path)
        }
    }

    /// Resolve a path and enforce the workspace boundary.
    ///
    /// Tools that touch the filesystem should use this instead of duplicating
    /// path resolution plus `is_safe_path` checks. Returns the checked absolute
    /// path, with existing ancestors canonicalized. Unresolved `..` is denied.
    fn resolve_workspace_path(&self, path: &str) -> Result<PathBuf, ToolError> {
        let resolved_path = self.resolve_path(path);
        resolve_safe_path(&resolved_path, self.working_directory()).ok_or_else(|| {
            ToolError::PermissionDenied(format!(
                "Access denied to path: {}",
                resolved_path.display()
            ))
        })
    }

    /// Check if a path is safe to access (within working directory)
    ///
    /// This method prevents path traversal attacks by ensuring the resolved
    /// path is within the working directory. It handles:
    /// - Absolute paths that point outside working directory
    /// - Relative paths with `..` components that escape the sandbox
    /// - Symlinks that point outside the working directory
    fn is_safe_path(&self, path: &Path) -> bool {
        resolve_safe_path(path, self.working_directory()).is_some()
    }
}

fn resolve_safe_path(path: &Path, working_directory: &Path) -> Option<PathBuf> {
    let working_dir = working_directory.canonicalize().ok()?;
    let mut current = path.to_path_buf();
    let mut components_to_add = Vec::new();

    loop {
        match std::fs::symlink_metadata(&current) {
            Ok(_) => {
                // Canonicalize existing ancestors, including symlinks. A dangling
                // symlink must fail here rather than be treated as a new file.
                let mut canonical = current.canonicalize().ok()?;
                for component in components_to_add.into_iter().rev() {
                    canonical.push(component);
                }
                return canonical.starts_with(&working_dir).then_some(canonical);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return None,
        }

        // Only ordinary names may be appended to the canonical ancestor.
        // ParentDir cannot be resolved safely past a missing directory.
        match current.components().next_back()? {
            Component::Normal(name) => components_to_add.push(name.to_os_string()),
            _ => return None,
        }
        if !current.pop() {
            return None;
        }
    }
}
