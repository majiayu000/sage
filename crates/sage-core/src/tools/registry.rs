//! Tool registry for managing available tools

use crate::tools::base::Tool;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    categories: HashMap<String, Vec<String>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            categories: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Register a tool with a category
    pub fn register_with_category(&mut self, tool: Arc<dyn Tool>, category: &str) {
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), tool);

        self.categories
            .entry(category.to_string())
            .or_insert_with(Vec::new)
            .push(name);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Get all tools in a category
    pub fn get_category(&self, category: &str) -> Vec<&Arc<dyn Tool>> {
        self.categories
            .get(category)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.tools.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get all category names
    pub fn category_names(&self) -> Vec<String> {
        self.categories.keys().cloned().collect()
    }

    /// Check if a tool is registered
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get tools by names
    pub fn get_tools(&self, names: &[String]) -> Vec<Arc<dyn Tool>> {
        names
            .iter()
            .filter_map(|name| self.tools.get(name).cloned())
            .collect()
    }

    /// Get all tools
    pub fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Remove a tool
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        // Remove from categories
        for tools in self.categories.values_mut() {
            tools.retain(|tool_name| tool_name != name);
        }

        self.tools.remove(name)
    }

    /// Clear all tools
    pub fn clear(&mut self) {
        self.tools.clear();
        self.categories.clear();
    }

    /// Get registry statistics
    pub fn statistics(&self) -> RegistryStatistics {
        RegistryStatistics {
            total_tools: self.tools.len(),
            total_categories: self.categories.len(),
            tools_by_category: self
                .categories
                .iter()
                .map(|(cat, tools)| (cat.clone(), tools.len()))
                .collect(),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the tool registry
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    /// Total number of registered tools
    pub total_tools: usize,
    /// Total number of categories
    pub total_categories: usize,
    /// Number of tools in each category
    pub tools_by_category: HashMap<String, usize>,
}

/// Builder for tool registry
pub struct ToolRegistryBuilder {
    tools: Vec<(Arc<dyn Tool>, Option<String>)>,
}

impl ToolRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Add a tool
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push((tool, None));
        self
    }

    /// Add a tool with category
    pub fn with_tool_in_category(mut self, tool: Arc<dyn Tool>, category: &str) -> Self {
        self.tools.push((tool, Some(category.to_string())));
        self
    }

    /// Add multiple tools
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        for tool in tools {
            self.tools.push((tool, None));
        }
        self
    }

    /// Build the registry
    pub fn build(self) -> ToolRegistry {
        let mut registry = ToolRegistry::new();

        for (tool, category) in self.tools {
            if let Some(cat) = category {
                registry.register_with_category(tool, &cat);
            } else {
                registry.register(tool);
            }
        }

        registry
    }
}

impl Default for ToolRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

use parking_lot::Mutex;
use std::sync::LazyLock;

/// Global tool registry instance using LazyLock (Rust 2024 edition)
/// Uses parking_lot::Mutex for non-poisoning, faster locks
static GLOBAL_REGISTRY: LazyLock<Mutex<ToolRegistry>> =
    LazyLock::new(|| Mutex::new(ToolRegistry::new()));

/// Get the global tool registry
pub fn with_global_registry<F, R>(f: F) -> R
where
    F: FnOnce(&mut ToolRegistry) -> R,
{
    let mut registry = GLOBAL_REGISTRY.lock();
    f(&mut *registry)
}

/// Register a tool globally
pub fn register_global_tool(tool: Arc<dyn Tool>) {
    with_global_registry(|registry| registry.register(tool));
}

/// Register a tool globally with category
pub fn register_global_tool_with_category(tool: Arc<dyn Tool>, category: &str) {
    with_global_registry(|registry| registry.register_with_category(tool, category));
}

/// Get a tool from the global registry
pub fn get_global_tool(name: &str) -> Option<Arc<dyn Tool>> {
    with_global_registry(|registry| registry.get(name).cloned())
}
