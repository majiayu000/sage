//! Sandbox environment for isolated task execution
//!
//! Provides a temporary, isolated directory for running evaluation tasks safely.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tempfile::TempDir;
use tokio::fs;

/// Sandbox environment for isolated task execution
pub struct Sandbox {
    /// Temporary directory (owned, will be cleaned up on drop)
    temp_dir: Option<TempDir>,

    /// Path to the sandbox root
    root: PathBuf,

    /// Whether to preserve the sandbox after completion
    preserve: bool,
}

impl Sandbox {
    /// Create a new sandbox with a temporary directory
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let root = temp_dir.path().to_path_buf();

        Ok(Self {
            temp_dir: Some(temp_dir),
            root,
            preserve: false,
        })
    }

    /// Create a sandbox at a specific path (not temporary)
    pub fn at_path(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).context("Failed to create sandbox directory")?;

        Ok(Self {
            temp_dir: None,
            root,
            preserve: true,
        })
    }

    /// Get the root path of the sandbox
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Set whether to preserve the sandbox after completion
    pub fn set_preserve(&mut self, preserve: bool) {
        self.preserve = preserve;
    }

    /// Set up initial files in the sandbox
    pub async fn setup_files(&self, files: &HashMap<String, String>) -> Result<()> {
        for (path, content) in files {
            let full_path = self.root.join(path);

            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            // Write the file
            fs::write(&full_path, content)
                .await
                .with_context(|| format!("Failed to write file: {:?}", full_path))?;

            tracing::debug!("Created file: {:?}", full_path);
        }

        Ok(())
    }

    /// Read a file from the sandbox
    pub async fn read_file(&self, path: impl AsRef<Path>) -> Result<String> {
        let full_path = self.root.join(path.as_ref());
        fs::read_to_string(&full_path)
            .await
            .with_context(|| format!("Failed to read file: {:?}", full_path))
    }

    /// Check if a file exists in the sandbox
    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        self.root.join(path.as_ref()).exists()
    }

    /// List all files in the sandbox
    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.list_files_recursive(&self.root, &mut files)?;
        Ok(files)
    }

    fn list_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    self.list_files_recursive(&path, files)?;
                } else {
                    // Store relative path
                    if let Ok(relative) = path.strip_prefix(&self.root) {
                        files.push(relative.to_path_buf());
                    }
                }
            }
        }
        Ok(())
    }

    /// Clean up the sandbox
    pub async fn cleanup(&mut self) -> Result<()> {
        if self.preserve {
            tracing::debug!("Preserving sandbox at {:?}", self.root);
            return Ok(());
        }

        if let Some(temp_dir) = self.temp_dir.take() {
            // TempDir will clean up on drop
            drop(temp_dir);
            tracing::debug!("Cleaned up sandbox");
        }

        Ok(())
    }

    /// Get the path for persisting the sandbox (moves out of temp)
    pub fn persist(mut self) -> Result<PathBuf> {
        if let Some(temp_dir) = self.temp_dir.take() {
            let path = temp_dir.keep();
            self.root = path.clone();
            self.preserve = true;
            Ok(path)
        } else {
            Ok(self.root.clone())
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if !self.preserve {
            // TempDir handles cleanup automatically
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_creation() {
        let sandbox = Sandbox::new().unwrap();
        assert!(sandbox.root().exists());
    }

    #[tokio::test]
    async fn test_sandbox_setup_files() {
        let sandbox = Sandbox::new().unwrap();

        let mut files = HashMap::new();
        files.insert("test.txt".to_string(), "Hello, World!".to_string());
        files.insert("src/main.rs".to_string(), "fn main() {}".to_string());

        sandbox.setup_files(&files).await.unwrap();

        assert!(sandbox.file_exists("test.txt"));
        assert!(sandbox.file_exists("src/main.rs"));

        let content = sandbox.read_file("test.txt").await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_sandbox_list_files() {
        let sandbox = Sandbox::new().unwrap();

        let mut files = HashMap::new();
        files.insert("a.txt".to_string(), "a".to_string());
        files.insert("b.txt".to_string(), "b".to_string());
        files.insert("dir/c.txt".to_string(), "c".to_string());

        sandbox.setup_files(&files).await.unwrap();

        let listed = sandbox.list_files().unwrap();
        assert_eq!(listed.len(), 3);
    }

    #[tokio::test]
    async fn test_sandbox_persist() {
        let sandbox = Sandbox::new().unwrap();
        let root = sandbox.root().to_path_buf();

        let mut files = HashMap::new();
        files.insert("test.txt".to_string(), "persist me".to_string());
        sandbox.setup_files(&files).await.unwrap();

        let persisted_path = sandbox.persist().unwrap();

        // File should still exist after persist
        assert!(persisted_path.join("test.txt").exists());

        // Clean up manually
        std::fs::remove_dir_all(&persisted_path).ok();
    }
}
