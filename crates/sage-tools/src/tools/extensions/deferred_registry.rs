//! Deferred tool registry for lazy-loaded tools

use sage_core::tools::base::Tool;
use std::collections::HashMap;
use std::sync::Arc;

/// Result from a tool search
#[derive(Debug, Clone)]
pub struct ToolSearchResult {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    /// Whether the tool is now loaded
    pub loaded: bool,
}

/// Registry of deferred tools that can be loaded on demand
pub struct DeferredToolRegistry {
    /// Available but not yet loaded tools
    available: HashMap<String, DeferredToolInfo>,
    /// Loaded tools
    loaded: HashMap<String, Arc<dyn Tool>>,
    /// Tool loader function
    loader: Option<Box<dyn Fn(&str) -> Option<Arc<dyn Tool>> + Send + Sync>>,
}

/// Information about a deferred tool
#[derive(Debug, Clone)]
pub struct DeferredToolInfo {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub source: String, // e.g., "mcp", "builtin", "custom"
}

impl Default for DeferredToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DeferredToolRegistry {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            loaded: HashMap::new(),
            loader: None,
        }
    }

    /// Register a deferred tool
    pub fn register_deferred(&mut self, info: DeferredToolInfo) {
        self.available.insert(info.name.clone(), info);
    }

    /// Set the tool loader function
    pub fn set_loader<F>(&mut self, loader: F)
    where
        F: Fn(&str) -> Option<Arc<dyn Tool>> + Send + Sync + 'static,
    {
        self.loader = Some(Box::new(loader));
    }

    /// Load a tool by name
    pub fn load(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        // Already loaded?
        if let Some(tool) = self.loaded.get(name) {
            return Some(Arc::clone(tool));
        }

        // Try to load
        if let Some(ref loader) = self.loader {
            if let Some(tool) = loader(name) {
                self.loaded.insert(name.to_string(), Arc::clone(&tool));
                self.available.remove(name);
                return Some(tool);
            }
        }

        None
    }

    /// Search for tools by keywords
    pub fn search(&self, query: &str, limit: usize) -> Vec<ToolSearchResult> {
        let keywords: Vec<&str> = query.split_whitespace().collect();
        if keywords.is_empty() {
            return vec![];
        }

        let mut results: Vec<(String, f64)> = self
            .available
            .iter()
            .map(|(name, info)| {
                let score = self.calculate_score(info, &keywords);
                (name.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        results
            .into_iter()
            .take(limit)
            .map(|(name, score)| {
                let info = self.available.get(&name).unwrap();
                ToolSearchResult {
                    name: name.clone(),
                    description: info.description.clone(),
                    score,
                    loaded: false,
                }
            })
            .collect()
    }

    /// Search with a required keyword
    pub fn search_with_required(
        &self,
        required: &str,
        other_keywords: &[&str],
        limit: usize,
    ) -> Vec<ToolSearchResult> {
        let mut results: Vec<(String, f64)> = self
            .available
            .iter()
            .filter(|(name, info)| {
                // Must match required keyword
                name.to_lowercase().contains(&required.to_lowercase())
                    || info
                        .keywords
                        .iter()
                        .any(|k| k.to_lowercase().contains(&required.to_lowercase()))
            })
            .map(|(name, info)| {
                let score = if other_keywords.is_empty() {
                    1.0
                } else {
                    self.calculate_score(info, other_keywords)
                };
                (name.clone(), score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results
            .into_iter()
            .take(limit)
            .map(|(name, score)| {
                let info = self.available.get(&name).unwrap();
                ToolSearchResult {
                    name: name.clone(),
                    description: info.description.clone(),
                    score,
                    loaded: false,
                }
            })
            .collect()
    }

    /// Calculate relevance score for a tool
    fn calculate_score(&self, info: &DeferredToolInfo, keywords: &[&str]) -> f64 {
        let mut score = 0.0;
        let name_lower = info.name.to_lowercase();
        let desc_lower = info.description.to_lowercase();

        for keyword in keywords {
            let kw_lower = keyword.to_lowercase();

            // Exact name match
            if name_lower == kw_lower {
                score += 1.0;
            }
            // Name contains keyword
            else if name_lower.contains(&kw_lower) {
                score += 0.7;
            }
            // Description contains keyword
            else if desc_lower.contains(&kw_lower) {
                score += 0.3;
            }
            // Keywords match
            else if info
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(&kw_lower))
            {
                score += 0.5;
            }
        }

        // Normalize by number of keywords
        score / keywords.len() as f64
    }

    /// Get a loaded tool
    pub fn get_loaded(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.loaded.get(name).cloned()
    }

    /// Check if a tool is available (deferred or loaded)
    pub fn is_available(&self, name: &str) -> bool {
        self.available.contains_key(name) || self.loaded.contains_key(name)
    }

    /// List all available tool names
    pub fn list_available(&self) -> Vec<String> {
        let mut names: Vec<String> = self.available.keys().cloned().collect();
        names.extend(self.loaded.keys().cloned());
        names.sort();
        names.dedup();
        names
    }
}
