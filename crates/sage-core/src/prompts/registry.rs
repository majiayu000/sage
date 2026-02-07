//! Prompt template registry
//!
//! Manages a collection of prompt templates with discovery and lookup.

use super::template::PromptTemplate;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Registry for prompt templates
#[derive(Debug, Default)]
pub struct PromptRegistry {
    /// Templates by name
    templates: HashMap<String, PromptTemplate>,
    /// Templates by category
    by_category: HashMap<String, Vec<String>>,
    /// Templates by tag
    by_tag: HashMap<String, Vec<String>>,
}

impl PromptRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Create registry with builtin prompts
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register_builtins();
        registry
    }

    /// Register a template
    pub fn register(&mut self, template: PromptTemplate) {
        let name = template.name.clone();

        // Index by category
        if let Some(ref category) = template.category {
            self.by_category
                .entry(category.clone())
                .or_default()
                .push(name.clone());
        }

        // Index by tags
        for tag in &template.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .push(name.clone());
        }

        self.templates.insert(name, template);
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// Get a template and render it
    pub fn render(&self, name: &str, values: &[(&str, &str)]) -> Option<String> {
        self.get(name).map(|t| t.render(values))
    }

    /// List all template names
    pub fn list(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    /// List templates by category
    pub fn list_by_category(&self, category: &str) -> Vec<&PromptTemplate> {
        self.by_category
            .get(category)
            .map(|names| names.iter().filter_map(|n| self.templates.get(n)).collect())
            .unwrap_or_default()
    }

    /// List templates by tag
    pub fn list_by_tag(&self, tag: &str) -> Vec<&PromptTemplate> {
        self.by_tag
            .get(tag)
            .map(|names| names.iter().filter_map(|n| self.templates.get(n)).collect())
            .unwrap_or_default()
    }

    /// List all categories
    pub fn categories(&self) -> Vec<&str> {
        self.by_category.keys().map(|s| s.as_str()).collect()
    }

    /// List all tags
    pub fn tags(&self) -> Vec<&str> {
        self.by_tag.keys().map(|s| s.as_str()).collect()
    }

    /// Search templates by keyword in name or description
    pub fn search(&self, query: &str) -> Vec<&PromptTemplate> {
        let query_lower = query.to_lowercase();
        self.templates
            .values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Remove a template
    pub fn remove(&mut self, name: &str) -> Option<PromptTemplate> {
        if let Some(template) = self.templates.remove(name) {
            // Clean up category index
            if let Some(ref category) = template.category {
                if let Some(names) = self.by_category.get_mut(category) {
                    names.retain(|n| n != name);
                }
            }

            // Clean up tag index
            for tag in &template.tags {
                if let Some(names) = self.by_tag.get_mut(tag) {
                    names.retain(|n| n != name);
                }
            }

            Some(template)
        } else {
            None
        }
    }

    /// Load templates from a directory
    pub async fn load_from_directory(&mut self, dir: &Path) -> std::io::Result<usize> {
        let mut count = 0;

        if !dir.exists() {
            return Ok(0);
        }

        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path
                .extension()
                .map(|e| e == "md" || e == "txt")
                .unwrap_or(false)
            {
                if let Ok(content) = fs::read_to_string(&path).await {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let template = PromptTemplate::new(name, content).with_category("custom");

                    self.register(template);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Get template count
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Register builtin prompts
    fn register_builtins(&mut self) {
        // System prompts
        self.register(
            PromptTemplate::new(
                "system_default",
                "You are {{role}}, a helpful AI assistant. {{additional_context}}",
            )
            .with_description("Default system prompt")
            .with_category("system")
            .with_default("role", "Claude")
            .with_default("additional_context", ""),
        );

        self.register(
            PromptTemplate::new(
                "system_coding",
                "You are an expert software engineer. You write clean, efficient, and well-documented code.\n\n\
                 Language preference: {{language}}\n\
                 Style guide: {{style_guide}}"
            )
            .with_description("System prompt for coding tasks")
            .with_category("system")
            .with_default("language", "the most appropriate language")
            .with_default("style_guide", "follow best practices")
        );

        // Task prompts
        self.register(
            PromptTemplate::new(
                "code_review",
                "Please review the following code for:\n\
                 - Code quality and best practices\n\
                 - Potential bugs or issues\n\
                 - Performance considerations\n\
                 - Security concerns\n\n\
                 Code:\n```{{language}}\n{{code}}\n```",
            )
            .with_description("Code review prompt")
            .with_category("task")
            .with_tags(vec!["code".to_string(), "review".to_string()])
            .with_required("code")
            .with_default("language", ""),
        );

        self.register(
            PromptTemplate::new(
                "explain_code",
                "Please explain the following {{language}} code:\n\n\
                 ```{{language}}\n{{code}}\n```\n\n\
                 Focus on:\n\
                 - What the code does\n\
                 - How it works\n\
                 - Key concepts used",
            )
            .with_description("Code explanation prompt")
            .with_category("task")
            .with_tags(vec!["code".to_string(), "explain".to_string()])
            .with_required("code")
            .with_default("language", ""),
        );

        self.register(
            PromptTemplate::new(
                "fix_error",
                "I'm getting the following error:\n\n\
                 ```\n{{error}}\n```\n\n\
                 In this code:\n```{{language}}\n{{code}}\n```\n\n\
                 Please help me understand and fix this error.",
            )
            .with_description("Error fixing prompt")
            .with_category("task")
            .with_tags(vec!["code".to_string(), "debug".to_string()])
            .with_required("error")
            .with_required("code")
            .with_default("language", ""),
        );

        self.register(
            PromptTemplate::new(
                "refactor",
                "Please refactor the following code to improve {{improvement_focus}}:\n\n\
                 ```{{language}}\n{{code}}\n```\n\n\
                 Requirements:\n{{requirements}}",
            )
            .with_description("Code refactoring prompt")
            .with_category("task")
            .with_tags(vec!["code".to_string(), "refactor".to_string()])
            .with_required("code")
            .with_default("language", "")
            .with_default("improvement_focus", "readability and maintainability")
            .with_default(
                "requirements",
                "- Maintain existing functionality\n- Add appropriate comments",
            ),
        );

        // Conversation prompts
        self.register(
            PromptTemplate::new(
                "summarize",
                "Please summarize the following text:\n\n{{text}}\n\n\
                 Summary length: {{length}}\n\
                 Focus on: {{focus}}",
            )
            .with_description("Text summarization prompt")
            .with_category("conversation")
            .with_required("text")
            .with_default("length", "concise")
            .with_default("focus", "key points"),
        );

        self.register(
            PromptTemplate::new(
                "translate",
                "Please translate the following from {{source_lang}} to {{target_lang}}:\n\n{{text}}"
            )
            .with_description("Translation prompt")
            .with_category("conversation")
            .with_required("text")
            .with_required("target_lang")
            .with_default("source_lang", "auto-detect")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic() {
        let mut registry = PromptRegistry::new();
        let template = PromptTemplate::new("test", "Hello {{name}}!");

        registry.register(template);

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn test_registry_render() {
        let mut registry = PromptRegistry::new();
        registry.register(PromptTemplate::new("greeting", "Hello {{name}}!"));

        let result = registry.render("greeting", &[("name", "World")]);
        assert_eq!(result, Some("Hello World!".to_string()));
    }

    #[test]
    fn test_registry_list() {
        let mut registry = PromptRegistry::new();
        registry.register(PromptTemplate::new("a", "A"));
        registry.register(PromptTemplate::new("b", "B"));

        let names = registry.list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_registry_by_category() {
        let mut registry = PromptRegistry::new();
        registry.register(PromptTemplate::new("a", "A").with_category("cat1"));
        registry.register(PromptTemplate::new("b", "B").with_category("cat1"));
        registry.register(PromptTemplate::new("c", "C").with_category("cat2"));

        let cat1 = registry.list_by_category("cat1");
        assert_eq!(cat1.len(), 2);

        let cat2 = registry.list_by_category("cat2");
        assert_eq!(cat2.len(), 1);
    }

    #[test]
    fn test_registry_by_tag() {
        let mut registry = PromptRegistry::new();
        registry.register(
            PromptTemplate::new("a", "A").with_tags(vec!["tag1".to_string(), "tag2".to_string()]),
        );
        registry.register(PromptTemplate::new("b", "B").with_tags(vec!["tag1".to_string()]));

        let tag1 = registry.list_by_tag("tag1");
        assert_eq!(tag1.len(), 2);

        let tag2 = registry.list_by_tag("tag2");
        assert_eq!(tag2.len(), 1);
    }

    #[test]
    fn test_registry_search() {
        let mut registry = PromptRegistry::new();
        registry.register(
            PromptTemplate::new("code_review", "Review code")
                .with_description("Review code for quality"),
        );
        registry
            .register(PromptTemplate::new("greeting", "Hello").with_description("Simple greeting"));

        let results = registry.search("code");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "code_review");

        let results2 = registry.search("greeting");
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_registry_remove() {
        let mut registry = PromptRegistry::new();
        registry.register(
            PromptTemplate::new("test", "Test")
                .with_category("cat")
                .with_tags(vec!["tag".to_string()]),
        );

        assert!(registry.get("test").is_some());

        let removed = registry.remove("test");
        assert!(removed.is_some());
        assert!(registry.get("test").is_none());
    }

    #[test]
    fn test_registry_builtins() {
        let registry = PromptRegistry::with_builtins();

        assert!(!registry.is_empty());
        assert!(registry.get("system_default").is_some());
        assert!(registry.get("code_review").is_some());
    }

    #[test]
    fn test_registry_categories() {
        let registry = PromptRegistry::with_builtins();

        let categories = registry.categories();
        assert!(categories.contains(&"system"));
        assert!(categories.contains(&"task"));
    }

    #[test]
    fn test_registry_tags() {
        let registry = PromptRegistry::with_builtins();

        let tags = registry.tags();
        assert!(tags.contains(&"code"));
    }

    #[test]
    fn test_registry_is_empty() {
        let registry = PromptRegistry::new();
        assert!(registry.is_empty());

        let registry_with_builtins = PromptRegistry::with_builtins();
        assert!(!registry_with_builtins.is_empty());
    }

    #[test]
    fn test_builtin_code_review() {
        let registry = PromptRegistry::with_builtins();
        let result = registry.render(
            "code_review",
            &[
                ("code", "fn main() { println!(\"Hello\"); }"),
                ("language", "rust"),
            ],
        );

        assert!(result.is_some());
        let rendered = result.unwrap();
        assert!(rendered.contains("fn main()"));
        assert!(rendered.contains("rust"));
    }
}
