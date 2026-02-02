//! Prompt loader module
//!
//! Provides file-based prompt loading with caching and hot reload support.
//!
//! # Architecture
//!
//! - `file_loader`: Loads prompts from markdown files with YAML frontmatter
//! - `cache`: TTL-based caching for parsed prompts
//! - `embedded`: Compile-time embedded prompts as fallback
//! - `watcher`: File system watcher for hot reload
//!
//! # Search Path Priority
//!
//! 1. Project-level: `.sage/prompts/`
//! 2. User-level: `~/.config/sage/prompts/`
//! 3. Embedded: Compile-time embedded prompts (fallback)
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::prompts::loader::PromptLoader;
//!
//! let loader = PromptLoader::new();
//!
//! // Load a system prompt
//! let identity = loader.load("system-prompt", "identity");
//!
//! // Load an agent prompt
//! let explore = loader.load("agent-prompt", "explore");
//!
//! // Load a tool description
//! let bash = loader.load("tool-description", "bash");
//! ```

mod cache;
mod embedded;
mod file_loader;
mod watcher;

pub use cache::PromptCache;
pub use embedded::EmbeddedPrompts;
pub use file_loader::{FileLoader, PromptFile, PromptMetadata};
pub use watcher::{PromptWatcher, SimpleWatcher};

use crate::prompts::template_engine::render;
use crate::prompts::PromptVariables;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

/// Main prompt loader with caching and fallback support
pub struct PromptLoader {
    /// File-based loader
    file_loader: Arc<RwLock<FileLoader>>,
    /// Embedded prompts fallback
    embedded: EmbeddedPrompts,
    /// Cache for rendered prompts
    cache: PromptCache,
    /// Whether to use file-based prompts (vs embedded only)
    use_file_prompts: bool,
}

impl std::fmt::Debug for PromptLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptLoader")
            .field("use_file_prompts", &self.use_file_prompts)
            .field("cache_size", &self.cache.len())
            .finish()
    }
}

impl PromptLoader {
    /// Create a new prompt loader with default settings
    pub fn new() -> Self {
        let mut file_loader = FileLoader::new();

        // Add default search paths
        for path in FileLoader::default_search_paths() {
            file_loader = file_loader.with_search_path(path);
        }

        // Try to load from file system
        let _ = file_loader.load_all();

        Self {
            file_loader: Arc::new(RwLock::new(file_loader)),
            embedded: EmbeddedPrompts::new(),
            cache: PromptCache::new(),
            use_file_prompts: true,
        }
    }

    /// Create a loader that only uses embedded prompts
    pub fn embedded_only() -> Self {
        Self {
            file_loader: Arc::new(RwLock::new(FileLoader::new())),
            embedded: EmbeddedPrompts::new(),
            cache: PromptCache::new(),
            use_file_prompts: false,
        }
    }

    /// Create a loader with custom search paths
    pub fn with_search_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut file_loader = FileLoader::new();
        for path in paths {
            file_loader = file_loader.with_search_path(path);
        }
        let _ = file_loader.load_all();

        Self {
            file_loader: Arc::new(RwLock::new(file_loader)),
            embedded: EmbeddedPrompts::new(),
            cache: PromptCache::new(),
            use_file_prompts: true,
        }
    }

    /// Load a prompt by category and name (raw content)
    pub fn load(&self, category: &str, name: &str) -> Option<String> {
        // Try file-based first if enabled
        if self.use_file_prompts {
            let loader = self.file_loader.read();
            if let Some(prompt) = loader.get(category, name) {
                return Some(prompt.content.clone());
            }
        }

        // Fall back to embedded
        self.embedded.get(category, name).map(|p| p.content.clone())
    }

    /// Load and render a prompt with variables
    pub fn load_rendered(
        &self,
        category: &str,
        name: &str,
        vars: &PromptVariables,
    ) -> Option<String> {
        let content = self.load(category, name)?;
        Some(render(&content, vars))
    }

    /// Load a prompt file (with metadata)
    pub fn load_file(&self, category: &str, name: &str) -> Option<PromptFile> {
        // Try file-based first if enabled
        if self.use_file_prompts {
            let loader = self.file_loader.read();
            if let Some(prompt) = loader.get(category, name) {
                return Some(prompt.clone());
            }
        }

        // Fall back to embedded
        self.embedded.get(category, name).cloned()
    }

    /// Load all prompts in a category
    pub fn load_category(&self, category: &str) -> Vec<PromptFile> {
        let mut prompts = Vec::new();

        // Get from file loader
        if self.use_file_prompts {
            let loader = self.file_loader.read();
            prompts.extend(loader.get_category(category).into_iter().cloned());
        }

        // Add embedded prompts not already loaded
        for embedded in self.embedded.get_category(category) {
            let key = format!("{}/{}", category, embedded.name());
            if !prompts.iter().any(|p| {
                let pk = format!("{}/{}", p.metadata.category, p.metadata.name);
                pk == key
            }) {
                prompts.push(embedded.clone());
            }
        }

        prompts
    }

    /// Check if a prompt exists
    pub fn exists(&self, category: &str, name: &str) -> bool {
        if self.use_file_prompts {
            let loader = self.file_loader.read();
            if loader.contains(category, name) {
                return true;
            }
        }
        self.embedded.contains(category, name)
    }

    /// List all available prompt keys
    pub fn list_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();

        // Add file-based keys
        if self.use_file_prompts {
            let loader = self.file_loader.read();
            keys.extend(loader.list_keys().into_iter().map(|s| s.to_string()));
        }

        // Add embedded keys not already present
        for key in self.embedded.list_keys() {
            if !keys.contains(&key.to_string()) {
                keys.push(key.to_string());
            }
        }

        keys.sort();
        keys
    }

    /// Reload prompts from file system
    pub fn reload(&self) -> anyhow::Result<()> {
        if self.use_file_prompts {
            let mut loader = self.file_loader.write();
            loader.reload()?;
        }
        self.cache.clear();
        Ok(())
    }

    /// Invalidate cache for a specific prompt
    pub fn invalidate(&self, category: &str, name: &str) {
        let key = format!("{}/{}", category, name);
        self.cache.invalidate(&key);
    }

    /// Invalidate all cached prompts in a category
    pub fn invalidate_category(&self, category: &str) {
        let prefix = format!("{}/", category);
        self.cache.invalidate_prefix(&prefix);
    }

    /// Clear all caches
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get the embedded prompts registry
    pub fn embedded(&self) -> &EmbeddedPrompts {
        &self.embedded
    }

    /// Add a custom search path and reload
    pub fn add_search_path(&self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        if self.use_file_prompts {
            let path = path.into();
            let mut loader = self.file_loader.write();
            *loader = std::mem::take(&mut *loader).with_search_path(path);
            loader.reload()?;
        }
        Ok(())
    }
}

impl Default for PromptLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PromptLoader {
    fn clone(&self) -> Self {
        Self {
            file_loader: Arc::clone(&self.file_loader),
            embedded: EmbeddedPrompts::new(), // Embedded is stateless
            cache: self.cache.clone(),
            use_file_prompts: self.use_file_prompts,
        }
    }
}

/// Global prompt loader instance
static GLOBAL_LOADER: std::sync::OnceLock<PromptLoader> = std::sync::OnceLock::new();

/// Get the global prompt loader
pub fn global_loader() -> &'static PromptLoader {
    GLOBAL_LOADER.get_or_init(PromptLoader::new)
}

/// Load a prompt using the global loader
pub fn load_prompt(category: &str, name: &str) -> Option<String> {
    global_loader().load(category, name)
}

/// Load and render a prompt using the global loader
pub fn load_prompt_rendered(category: &str, name: &str, vars: &PromptVariables) -> Option<String> {
    global_loader().load_rendered(category, name, vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = PromptLoader::new();
        assert!(!loader.list_keys().is_empty());
    }

    #[test]
    fn test_embedded_only_loader() {
        let loader = PromptLoader::embedded_only();
        assert!(!loader.list_keys().is_empty());
    }

    #[test]
    fn test_load_system_prompt() {
        let loader = PromptLoader::new();
        let content = loader.load("system-prompt", "identity");
        assert!(content.is_some());
        assert!(content.unwrap().contains("${AGENT_NAME}"));
    }

    #[test]
    fn test_load_agent_prompt() {
        let loader = PromptLoader::new();
        let content = loader.load("agent-prompt", "explore");
        assert!(content.is_some());
        assert!(content.unwrap().contains("READ-ONLY"));
    }

    #[test]
    fn test_load_tool_description() {
        let loader = PromptLoader::new();
        let content = loader.load("tool-description", "bash");
        assert!(content.is_some());
    }

    #[test]
    fn test_load_rendered() {
        let loader = PromptLoader::new();
        let vars = PromptVariables::new();
        let content = loader.load_rendered("system-prompt", "identity", &vars);
        assert!(content.is_some());
        assert!(content.unwrap().contains("Sage Agent"));
    }

    #[test]
    fn test_exists() {
        let loader = PromptLoader::new();
        assert!(loader.exists("system-prompt", "identity"));
        assert!(loader.exists("agent-prompt", "explore"));
        assert!(!loader.exists("nonexistent", "prompt"));
    }

    #[test]
    fn test_load_category() {
        let loader = PromptLoader::new();
        let prompts = loader.load_category("system-prompt");
        assert!(!prompts.is_empty());
    }

    #[test]
    fn test_global_loader() {
        let content = load_prompt("system-prompt", "identity");
        assert!(content.is_some());
    }
}
