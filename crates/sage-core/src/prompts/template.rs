//! Prompt template implementation
//!
//! Provides variable substitution and template rendering.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// A prompt template with variable substitution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Template name/identifier
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Raw template content with {{variable}} placeholders
    pub content: String,
    /// Variable definitions
    pub variables: Vec<PromptVariable>,
    /// Template category
    pub category: Option<String>,
    /// Tags for organization
    pub tags: Vec<String>,
}

impl PromptTemplate {
    /// Create a new template
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let variables = Self::extract_variables(&content);

        Self {
            name: name.into(),
            description: None,
            content,
            variables,
            category: None,
            tags: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set variable with default value
    pub fn with_default(mut self, name: &str, default: impl Into<String>) -> Self {
        if let Some(var) = self.variables.iter_mut().find(|v| v.name == name) {
            var.default = Some(default.into());
        }
        self
    }

    /// Mark variable as required
    pub fn with_required(mut self, name: &str) -> Self {
        if let Some(var) = self.variables.iter_mut().find(|v| v.name == name) {
            var.required = true;
        }
        self
    }

    /// Extract variables from template content
    fn extract_variables(content: &str) -> Vec<PromptVariable> {
        let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();
        let mut seen = std::collections::HashSet::new();
        let mut variables = Vec::new();

        for cap in re.captures_iter(content) {
            let name = cap[1].to_string();
            if seen.insert(name.clone()) {
                variables.push(PromptVariable {
                    name,
                    description: None,
                    default: None,
                    required: false,
                });
            }
        }

        variables
    }

    /// Render template with provided values
    pub fn render(&self, values: &[(&str, &str)]) -> String {
        let map: HashMap<&str, &str> = values.iter().cloned().collect();
        self.render_map(&map)
    }

    /// Render template with HashMap
    pub fn render_map(&self, values: &HashMap<&str, &str>) -> String {
        let mut result = self.content.clone();

        for var in &self.variables {
            let placeholder = format!("{{{{{}}}}}", var.name);
            let value = values
                .get(var.name.as_str())
                .copied()
                .or(var.default.as_deref())
                .unwrap_or("");

            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Render with validation
    pub fn render_validated(&self, values: &HashMap<&str, &str>) -> Result<String, RenderError> {
        // Check required variables
        for var in &self.variables {
            if var.required && !values.contains_key(var.name.as_str()) && var.default.is_none() {
                return Err(RenderError::MissingRequired(var.name.clone()));
            }
        }

        Ok(self.render_map(values))
    }

    /// Get list of required variables without defaults
    pub fn required_variables(&self) -> Vec<&PromptVariable> {
        self.variables
            .iter()
            .filter(|v| v.required && v.default.is_none())
            .collect()
    }

    /// Check if template has all required values
    pub fn has_required(&self, values: &HashMap<&str, &str>) -> bool {
        self.required_variables()
            .iter()
            .all(|v| values.contains_key(v.name.as_str()))
    }

    /// Get variable names
    pub fn variable_names(&self) -> Vec<&str> {
        self.variables.iter().map(|v| v.name.as_str()).collect()
    }
}

/// Variable definition in a template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariable {
    /// Variable name
    pub name: String,
    /// Description of the variable
    pub description: Option<String>,
    /// Default value
    pub default: Option<String>,
    /// Whether the variable is required
    pub required: bool,
}

impl PromptVariable {
    /// Create a new variable
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            default: None,
            required: false,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// Errors during template rendering
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Missing required variable: {0}")]
    MissingRequired(String),

    #[error("Invalid variable name: {0}")]
    InvalidVariable(String),

    #[error("Template parsing error: {0}")]
    ParseError(String),
}

/// Builder for creating templates
pub struct TemplateBuilder {
    name: String,
    content: String,
    description: Option<String>,
    category: Option<String>,
    tags: Vec<String>,
    defaults: HashMap<String, String>,
    required: Vec<String>,
}

impl TemplateBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: String::new(),
            description: None,
            category: None,
            tags: Vec::new(),
            defaults: HashMap::new(),
            required: Vec::new(),
        }
    }

    /// Set content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set category
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Add tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set default for variable
    pub fn default(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(name.into(), value.into());
        self
    }

    /// Mark variable as required
    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }

    /// Build the template
    pub fn build(self) -> PromptTemplate {
        let mut template = PromptTemplate::new(self.name, self.content);

        template.description = self.description;
        template.category = self.category;
        template.tags = self.tags;

        for (name, default) in self.defaults {
            if let Some(var) = template.variables.iter_mut().find(|v| v.name == name) {
                var.default = Some(default);
            }
        }

        for name in self.required {
            if let Some(var) = template.variables.iter_mut().find(|v| v.name == name) {
                var.required = true;
            }
        }

        template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_basic() {
        let template = PromptTemplate::new("test", "Hello {{name}}!");
        let result = template.render(&[("name", "World")]);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_template_multiple_variables() {
        let template = PromptTemplate::new("test", "{{greeting}} {{name}}, welcome to {{place}}!");
        let result =
            template.render(&[("greeting", "Hello"), ("name", "Alice"), ("place", "Sage")]);
        assert_eq!(result, "Hello Alice, welcome to Sage!");
    }

    #[test]
    fn test_template_repeated_variable() {
        let template = PromptTemplate::new("test", "{{name}} said: Hello {{name}}!");
        let result = template.render(&[("name", "Bob")]);
        assert_eq!(result, "Bob said: Hello Bob!");
    }

    #[test]
    fn test_template_default_value() {
        let template = PromptTemplate::new("test", "Hello {{name}}!").with_default("name", "World");
        let result = template.render(&[]);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_template_override_default() {
        let template = PromptTemplate::new("test", "Hello {{name}}!").with_default("name", "World");
        let result = template.render(&[("name", "Alice")]);
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_template_extract_variables() {
        let template = PromptTemplate::new("test", "{{a}} {{b}} {{c}} {{a}}");
        assert_eq!(template.variables.len(), 3);
        assert_eq!(template.variable_names(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_template_validated_missing_required() {
        let template = PromptTemplate::new("test", "Hello {{name}}!").with_required("name");
        let values: HashMap<&str, &str> = HashMap::new();
        let result = template.render_validated(&values);
        assert!(matches!(result, Err(RenderError::MissingRequired(_))));
    }

    #[test]
    fn test_template_validated_success() {
        let template = PromptTemplate::new("test", "Hello {{name}}!").with_required("name");
        let mut values = HashMap::new();
        values.insert("name", "World");
        let result = template.render_validated(&values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World!");
    }

    #[test]
    fn test_template_with_metadata() {
        let template = PromptTemplate::new("test", "{{content}}")
            .with_description("A test template")
            .with_category("testing")
            .with_tags(vec!["test".to_string(), "example".to_string()]);

        assert_eq!(template.description, Some("A test template".to_string()));
        assert_eq!(template.category, Some("testing".to_string()));
        assert_eq!(template.tags.len(), 2);
    }

    #[test]
    fn test_template_builder() {
        let template = TemplateBuilder::new("greeting")
            .content("{{greeting}} {{name}}!")
            .description("A greeting template")
            .category("greetings")
            .tag("common")
            .default("greeting", "Hello")
            .required("name")
            .build();

        assert_eq!(template.name, "greeting");
        assert!(
            template
                .variables
                .iter()
                .any(|v| v.name == "greeting" && v.default.is_some())
        );
        assert!(
            template
                .variables
                .iter()
                .any(|v| v.name == "name" && v.required)
        );
    }

    #[test]
    fn test_template_has_required() {
        let template = PromptTemplate::new("test", "{{a}} {{b}}").with_required("a");

        let mut values = HashMap::new();
        assert!(!template.has_required(&values));

        values.insert("a", "value");
        assert!(template.has_required(&values));
    }

    #[test]
    fn test_template_required_with_default() {
        let template = PromptTemplate::new("test", "{{a}} {{b}}")
            .with_required("a")
            .with_default("a", "default");

        // Required with default doesn't need to be provided
        let values: HashMap<&str, &str> = HashMap::new();
        let result = template.render_validated(&values);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_variable() {
        let var = PromptVariable::new("test")
            .with_description("A test variable")
            .with_default("default")
            .required();

        assert_eq!(var.name, "test");
        assert_eq!(var.description, Some("A test variable".to_string()));
        assert_eq!(var.default, Some("default".to_string()));
        assert!(var.required);
    }

    #[test]
    fn test_template_no_variables() {
        let template = PromptTemplate::new("static", "This is a static prompt with no variables.");
        assert!(template.variables.is_empty());
        assert_eq!(
            template.render(&[]),
            "This is a static prompt with no variables."
        );
    }

    #[test]
    fn test_template_missing_variable_renders_empty() {
        let template = PromptTemplate::new("test", "Hello {{name}}!");
        let result = template.render(&[]);
        assert_eq!(result, "Hello !");
    }
}
