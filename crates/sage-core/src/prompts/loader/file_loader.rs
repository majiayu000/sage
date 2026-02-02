//! Prompt file loader
//!
//! Loads prompt templates from markdown files with YAML frontmatter.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parsed prompt file with frontmatter and content
#[derive(Debug, Clone)]
pub struct PromptFile {
    /// Frontmatter metadata
    pub metadata: PromptMetadata,
    /// The prompt content (after frontmatter)
    pub content: String,
    /// Source file path
    pub source_path: Option<PathBuf>,
}

/// Prompt file frontmatter metadata
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PromptMetadata {
    /// Prompt name/identifier
    pub name: String,
    /// Description of the prompt
    #[serde(default)]
    pub description: String,
    /// Version string
    #[serde(default = "default_version")]
    pub version: String,
    /// Category (system-prompt, agent-prompt, tool-description, etc.)
    #[serde(default)]
    pub category: String,
    /// Variables used in this prompt
    #[serde(default)]
    pub variables: Vec<String>,
    /// Conditions that affect this prompt
    #[serde(default)]
    pub conditions: Vec<String>,
    /// Whether this is a read-only agent prompt
    #[serde(default)]
    pub read_only: Option<bool>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl PromptFile {
    /// Parse a prompt file from content
    pub fn parse(content: &str) -> Result<Self> {
        Self::parse_with_path(content, None)
    }

    /// Parse a prompt file from content with source path
    pub fn parse_with_path(content: &str, source_path: Option<PathBuf>) -> Result<Self> {
        let (metadata, prompt_content) = Self::parse_frontmatter(content)?;
        Ok(Self {
            metadata,
            content: prompt_content,
            source_path,
        })
    }

    /// Parse YAML frontmatter from content
    fn parse_frontmatter(content: &str) -> Result<(PromptMetadata, String)> {
        let content = content.trim();

        // Check for frontmatter delimiter
        if !content.starts_with("---") {
            // No frontmatter, return default metadata and full content
            return Ok((PromptMetadata::default(), content.to_string()));
        }

        // Find the end of frontmatter
        let rest = &content[3..];
        let end_pos = rest
            .find("\n---")
            .context("Invalid frontmatter: missing closing ---")?;

        let frontmatter_str = &rest[..end_pos].trim();
        let prompt_content = rest[end_pos + 4..].trim();

        // Parse YAML frontmatter
        let metadata: PromptMetadata =
            serde_yaml::from_str(frontmatter_str).context("Failed to parse frontmatter YAML")?;

        Ok((metadata, prompt_content.to_string()))
    }

    /// Load a prompt file from disk
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read prompt file: {}", path.display()))?;
        Self::parse_with_path(&content, Some(path.to_path_buf()))
    }

    /// Get the prompt name (from metadata or filename)
    pub fn name(&self) -> &str {
        if !self.metadata.name.is_empty() {
            &self.metadata.name
        } else if let Some(ref path) = self.source_path {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        } else {
            "unknown"
        }
    }
}

/// File-based prompt loader
pub struct FileLoader {
    /// Search paths for prompt files (in priority order)
    search_paths: Vec<PathBuf>,
    /// Loaded prompts cache
    prompts: HashMap<String, PromptFile>,
}

impl FileLoader {
    /// Create a new file loader with default search paths
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            prompts: HashMap::new(),
        }
    }

    /// Add a search path
    pub fn with_search_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.search_paths.push(path.into());
        self
    }

    /// Add multiple search paths
    pub fn with_search_paths(mut self, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        self.search_paths.extend(paths);
        self
    }

    /// Get default search paths
    pub fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Project-level: .sage/prompts/
        if let Ok(cwd) = std::env::current_dir() {
            paths.push(cwd.join(".sage").join("prompts"));
        }

        // 2. User-level: ~/.config/sage/prompts/
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("sage").join("prompts"));
        }

        // 3. Package-level: relative to crate
        // This will be handled by embedded prompts as fallback

        paths
    }

    /// Load all prompts from search paths
    pub fn load_all(&mut self) -> Result<()> {
        for search_path in &self.search_paths.clone() {
            if search_path.exists() {
                self.load_directory(search_path)?;
            }
        }
        Ok(())
    }

    /// Load prompts from a directory recursively
    pub fn load_directory(&mut self, dir: impl AsRef<Path>) -> Result<()> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.load_directory(&path)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(prompt) = PromptFile::load(&path) {
                    let key = self.make_key(&prompt, &path);
                    self.prompts.insert(key, prompt);
                }
            }
        }

        Ok(())
    }

    /// Create a cache key for a prompt
    fn make_key(&self, prompt: &PromptFile, path: &Path) -> String {
        // Use category/name format if available
        if !prompt.metadata.category.is_empty() && !prompt.metadata.name.is_empty() {
            format!("{}/{}", prompt.metadata.category, prompt.metadata.name)
        } else {
            // Fall back to relative path
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        }
    }

    /// Get a prompt by category and name
    pub fn get(&self, category: &str, name: &str) -> Option<&PromptFile> {
        let key = format!("{}/{}", category, name);
        self.prompts.get(&key)
    }

    /// Get a prompt by key
    pub fn get_by_key(&self, key: &str) -> Option<&PromptFile> {
        self.prompts.get(key)
    }

    /// Get all prompts in a category
    pub fn get_category(&self, category: &str) -> Vec<&PromptFile> {
        let prefix = format!("{}/", category);
        self.prompts
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v)
            .collect()
    }

    /// List all loaded prompt keys
    pub fn list_keys(&self) -> Vec<&str> {
        self.prompts.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a prompt exists
    pub fn contains(&self, category: &str, name: &str) -> bool {
        let key = format!("{}/{}", category, name);
        self.prompts.contains_key(&key)
    }

    /// Get the number of loaded prompts
    pub fn len(&self) -> usize {
        self.prompts.len()
    }

    /// Check if no prompts are loaded
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }

    /// Clear all loaded prompts
    pub fn clear(&mut self) {
        self.prompts.clear();
    }

    /// Reload all prompts
    pub fn reload(&mut self) -> Result<()> {
        self.clear();
        self.load_all()
    }
}

impl Default for FileLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
name: test-prompt
description: A test prompt
version: "1.0.0"
category: system-prompt
variables:
  - AGENT_NAME
  - BASH_TOOL_NAME
---

This is the prompt content.
Use ${AGENT_NAME} to refer to the agent.
"#;

        let prompt = PromptFile::parse(content).unwrap();
        assert_eq!(prompt.metadata.name, "test-prompt");
        assert_eq!(prompt.metadata.description, "A test prompt");
        assert_eq!(prompt.metadata.version, "1.0.0");
        assert_eq!(prompt.metadata.category, "system-prompt");
        assert_eq!(prompt.metadata.variables.len(), 2);
        assert!(prompt.content.contains("This is the prompt content"));
        assert!(prompt.content.contains("${AGENT_NAME}"));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "Just plain content without frontmatter.";
        let prompt = PromptFile::parse(content).unwrap();
        assert!(prompt.metadata.name.is_empty());
        assert_eq!(prompt.content, content);
    }

    #[test]
    fn test_parse_empty_frontmatter() {
        let content = r#"---
name: minimal
---

Content here."#;

        let prompt = PromptFile::parse(content).unwrap();
        assert_eq!(prompt.metadata.name, "minimal");
        assert_eq!(prompt.metadata.version, "1.0.0"); // default
        assert!(prompt.content.contains("Content here"));
    }

    #[test]
    fn test_file_loader_key_generation() {
        let loader = FileLoader::new();
        let prompt = PromptFile {
            metadata: PromptMetadata {
                name: "identity".to_string(),
                category: "system-prompt".to_string(),
                ..Default::default()
            },
            content: "test".to_string(),
            source_path: Some(PathBuf::from("/test/identity.md")),
        };

        let key = loader.make_key(&prompt, Path::new("/test/identity.md"));
        assert_eq!(key, "system-prompt/identity");
    }
}
