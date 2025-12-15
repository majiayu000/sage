//! Validation schema definitions

use super::rules::ValidationRule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Field type for validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    /// String type
    String,
    /// Integer type (i64)
    Integer,
    /// Number type (f64)
    Number,
    /// Boolean type
    Boolean,
    /// Array type
    Array,
    /// Object type
    Object,
    /// Any type (no type checking)
    Any,
    /// Null type
    Null,
    /// Union of multiple types
    Union(Vec<FieldType>),
}

impl FieldType {
    /// Check if a JSON value matches this type
    pub fn matches(&self, value: &serde_json::Value) -> bool {
        match self {
            FieldType::String => value.is_string(),
            FieldType::Integer => value.is_i64() || value.is_u64(),
            FieldType::Number => value.is_number(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::Array => value.is_array(),
            FieldType::Object => value.is_object(),
            FieldType::Any => true,
            FieldType::Null => value.is_null(),
            FieldType::Union(types) => types.iter().any(|t| t.matches(value)),
        }
    }

    /// Get the type name for error messages
    pub fn type_name(&self) -> &str {
        match self {
            FieldType::String => "string",
            FieldType::Integer => "integer",
            FieldType::Number => "number",
            FieldType::Boolean => "boolean",
            FieldType::Array => "array",
            FieldType::Object => "object",
            FieldType::Any => "any",
            FieldType::Null => "null",
            FieldType::Union(_) => "union",
        }
    }
}

/// Schema for a single field
#[derive(Debug, Clone)]
pub struct FieldSchema {
    /// Field type
    pub field_type: FieldType,

    /// Whether field is required
    required: bool,

    /// Default value if not provided
    pub default: Option<serde_json::Value>,

    /// Description of the field
    pub description: Option<String>,

    /// Validation rules
    pub rules: Vec<ValidationRule>,

    /// Nested schema for objects
    pub nested_schema: Option<Box<ValidationSchema>>,

    /// Item schema for arrays
    pub item_schema: Option<Box<FieldSchema>>,

    /// Enum values (for string fields)
    pub enum_values: Option<Vec<String>>,
}

impl FieldSchema {
    /// Create a new field schema
    pub fn new(field_type: FieldType) -> Self {
        Self {
            field_type,
            required: false,
            default: None,
            description: None,
            rules: Vec::new(),
            nested_schema: None,
            item_schema: None,
            enum_values: None,
        }
    }

    /// Set required status
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Check if field is required
    pub fn is_required(&self) -> bool {
        self.required
    }

    /// Set default value
    pub fn default_value(mut self, value: serde_json::Value) -> Self {
        self.default = Some(value);
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add validation rule
    pub fn rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add multiple validation rules
    pub fn rules(mut self, rules: Vec<ValidationRule>) -> Self {
        self.rules.extend(rules);
        self
    }

    /// Set nested schema for object types
    pub fn nested(mut self, schema: ValidationSchema) -> Self {
        self.nested_schema = Some(Box::new(schema));
        self
    }

    /// Set item schema for array types
    pub fn items(mut self, schema: FieldSchema) -> Self {
        self.item_schema = Some(Box::new(schema));
        self
    }

    /// Set enum values for string fields
    pub fn enum_of(mut self, values: Vec<impl Into<String>>) -> Self {
        self.enum_values = Some(values.into_iter().map(Into::into).collect());
        self
    }

    // Convenience methods for common constraints

    /// Add minimum length constraint (for strings)
    pub fn min_length(self, min: usize) -> Self {
        self.rule(ValidationRule::MinLength(min))
    }

    /// Add maximum length constraint (for strings)
    pub fn max_length(self, max: usize) -> Self {
        self.rule(ValidationRule::MaxLength(max))
    }

    /// Add minimum value constraint (for numbers)
    pub fn min_value(self, min: f64) -> Self {
        self.rule(ValidationRule::MinValue(min))
    }

    /// Add maximum value constraint (for numbers)
    pub fn max_value(self, max: f64) -> Self {
        self.rule(ValidationRule::MaxValue(max))
    }

    /// Add pattern constraint (regex for strings)
    pub fn pattern(self, pattern: impl Into<String>) -> Self {
        self.rule(ValidationRule::Pattern(pattern.into()))
    }

    /// Add email format validation
    pub fn email(self) -> Self {
        self.rule(ValidationRule::Email)
    }

    /// Add URL format validation
    pub fn url(self) -> Self {
        self.rule(ValidationRule::Url)
    }

    /// Add path format validation
    pub fn path(self) -> Self {
        self.rule(ValidationRule::Path)
    }

    /// Add non-empty constraint
    pub fn non_empty(self) -> Self {
        self.rule(ValidationRule::NonEmpty)
    }
}

/// Complete validation schema
#[derive(Debug, Clone)]
pub struct ValidationSchema {
    /// Field schemas
    pub fields: HashMap<String, FieldSchema>,

    /// Required field names
    pub required: Vec<String>,

    /// Whether to allow fields not in schema
    pub allow_extra_fields: bool,
}

impl ValidationSchema {
    /// Create an empty schema
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            required: Vec::new(),
            allow_extra_fields: false,
        }
    }

    /// Add a field to the schema
    pub fn add_field(&mut self, name: impl Into<String>, schema: FieldSchema) {
        let name = name.into();
        if schema.is_required() {
            self.required.push(name.clone());
        }
        self.fields.insert(name, schema);
    }

    /// Get a field schema by name
    pub fn get_field(&self, name: &str) -> Option<&FieldSchema> {
        self.fields.get(name)
    }

    /// Check if a field is required
    pub fn is_field_required(&self, name: &str) -> bool {
        self.required.contains(&name.to_string())
    }

    /// Check if extra fields are allowed
    pub fn allows_extra_fields(&self) -> bool {
        self.allow_extra_fields
    }
}

impl Default for ValidationSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_matches() {
        assert!(FieldType::String.matches(&serde_json::json!("hello")));
        assert!(!FieldType::String.matches(&serde_json::json!(123)));

        assert!(FieldType::Integer.matches(&serde_json::json!(42)));
        assert!(!FieldType::Integer.matches(&serde_json::json!(3.14)));

        assert!(FieldType::Number.matches(&serde_json::json!(3.14)));
        assert!(FieldType::Number.matches(&serde_json::json!(42)));

        assert!(FieldType::Boolean.matches(&serde_json::json!(true)));
        assert!(!FieldType::Boolean.matches(&serde_json::json!("true")));

        assert!(FieldType::Array.matches(&serde_json::json!([1, 2, 3])));
        assert!(FieldType::Object.matches(&serde_json::json!({"key": "value"})));

        assert!(FieldType::Any.matches(&serde_json::json!("anything")));
        assert!(FieldType::Any.matches(&serde_json::json!(null)));

        assert!(FieldType::Null.matches(&serde_json::json!(null)));
        assert!(!FieldType::Null.matches(&serde_json::json!("")));
    }

    #[test]
    fn test_union_type() {
        let union = FieldType::Union(vec![FieldType::String, FieldType::Integer]);

        assert!(union.matches(&serde_json::json!("hello")));
        assert!(union.matches(&serde_json::json!(42)));
        assert!(!union.matches(&serde_json::json!(true)));
    }

    #[test]
    fn test_field_schema_builder() {
        let schema = FieldSchema::new(FieldType::String)
            .required(true)
            .description("User name")
            .min_length(1)
            .max_length(100);

        assert!(schema.is_required());
        assert_eq!(schema.description, Some("User name".to_string()));
        assert_eq!(schema.rules.len(), 2);
    }

    #[test]
    fn test_validation_schema() {
        let mut schema = ValidationSchema::new();
        schema.add_field(
            "name",
            FieldSchema::new(FieldType::String).required(true),
        );
        schema.add_field(
            "age",
            FieldSchema::new(FieldType::Integer).required(false),
        );

        assert!(schema.is_field_required("name"));
        assert!(!schema.is_field_required("age"));
        assert_eq!(schema.fields.len(), 2);
    }

    #[test]
    fn test_nested_schema() {
        let address_schema = ValidationSchema::new();
        let field = FieldSchema::new(FieldType::Object).nested(address_schema);

        assert!(field.nested_schema.is_some());
    }

    #[test]
    fn test_enum_values() {
        let field = FieldSchema::new(FieldType::String)
            .enum_of(vec!["active", "inactive", "pending"]);

        assert!(field.enum_values.is_some());
        assert_eq!(field.enum_values.as_ref().unwrap().len(), 3);
    }
}
