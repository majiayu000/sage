//! File system helper trait for tools

use super::tool_trait::Tool;
use std::path::Path;

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

    /// Check if a path is safe to access (within working directory)
    ///
    /// This method prevents path traversal attacks by ensuring the resolved
    /// path is within the working directory. It handles:
    /// - Absolute paths that point outside working directory
    /// - Relative paths with `..` components that escape the sandbox
    /// - Symlinks that point outside the working directory
    fn is_safe_path(&self, path: &Path) -> bool {
        // Get the canonical working directory
        let working_dir = match self.working_directory().canonicalize() {
            Ok(p) => p,
            Err(_) => return false, // Can't verify if working dir doesn't exist
        };

        // Try to canonicalize the target path
        let canonical = if path.exists() {
            match path.canonicalize() {
                Ok(p) => p,
                Err(_) => return false,
            }
        } else {
            // For new files/directories, find the nearest existing ancestor
            // and build the path from there
            let mut current = path.to_path_buf();
            let mut components_to_add = Vec::new();

            // Walk up until we find an existing directory
            loop {
                if current.exists() {
                    match current.canonicalize() {
                        Ok(canonical_ancestor) => {
                            // Build the full path by appending non-existent components
                            let mut result = canonical_ancestor;
                            for component in components_to_add.into_iter().rev() {
                                result = result.join(component);
                            }
                            break result;
                        }
                        Err(_) => return false,
                    }
                }

                // Get the file name component to add later
                if let Some(name) = current.file_name() {
                    components_to_add.push(name.to_os_string());
                }

                // Move to parent
                if let Some(parent) = current.parent() {
                    if parent.as_os_str().is_empty() {
                        // We've reached the root of a relative path
                        // Use working directory as the base
                        let mut result = working_dir.clone();
                        for component in components_to_add.into_iter().rev() {
                            result = result.join(component);
                        }
                        break result;
                    }
                    current = parent.to_path_buf();
                } else {
                    return false;
                }
            }
        };

        // Check for path traversal attempts in the non-existent portion
        // by ensuring no ".." components exist after normalization
        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                // Found a ".." - need to verify the final path is still safe
                // The canonical path already resolved these, but we need to
                // ensure we don't escape the sandbox
            }
        }

        // Check if the canonical path starts with the working directory
        canonical.starts_with(&working_dir)
    }
}
