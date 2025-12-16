//! Input validation framework for tool arguments
//!
//! Provides comprehensive validation for tool inputs including:
//! - Type validation (string, number, boolean, array, object)
//! - Format validation (email, url, path, regex patterns)
//! - Range validation (min/max for numbers and strings)
//! - Custom validators
//! - Sanitization of dangerous inputs

mod rules;
mod sanitizer;
mod schema;
mod validator;

pub use rules::{CommonRules, RuleSet, ValidationRule};
pub use sanitizer::{InputSanitizer, SanitizeOptions};
pub use schema::{FieldSchema, FieldType, ValidationSchema};
pub use validator::{FieldError, ValidationError, ValidationResult, Validator};

use serde_json::Value;
use std::collections::HashMap;

/// Quick validation of a JSON value against a schema
pub fn validate(value: &Value, schema: &ValidationSchema) -> ValidationResult {
    Validator::new().validate(value, schema)
}

/// Quick sanitization of a JSON value
pub fn sanitize(value: &Value, options: &SanitizeOptions) -> Value {
    InputSanitizer::new(options.clone()).sanitize(value)
}

/// Builder for creating validation schemas
pub struct SchemaBuilder {
    fields: HashMap<String, FieldSchema>,
    required: Vec<String>,
    allow_extra: bool,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            required: Vec::new(),
            allow_extra: false,
        }
    }

    /// Add a required string field
    pub fn string(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields.insert(
            name.clone(),
            FieldSchema::new(FieldType::String).required(true),
        );
        self.required.push(name);
        self
    }

    /// Add an optional string field
    pub fn optional_string(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields
            .insert(name, FieldSchema::new(FieldType::String).required(false));
        self
    }

    /// Add a required integer field
    pub fn integer(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields.insert(
            name.clone(),
            FieldSchema::new(FieldType::Integer).required(true),
        );
        self.required.push(name);
        self
    }

    /// Add an optional integer field
    pub fn optional_integer(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields
            .insert(name, FieldSchema::new(FieldType::Integer).required(false));
        self
    }

    /// Add a required number field
    pub fn number(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields.insert(
            name.clone(),
            FieldSchema::new(FieldType::Number).required(true),
        );
        self.required.push(name);
        self
    }

    /// Add a required boolean field
    pub fn boolean(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields.insert(
            name.clone(),
            FieldSchema::new(FieldType::Boolean).required(true),
        );
        self.required.push(name);
        self
    }

    /// Add an optional boolean field
    pub fn optional_boolean(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields
            .insert(name, FieldSchema::new(FieldType::Boolean).required(false));
        self
    }

    /// Add a required array field
    pub fn array(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields.insert(
            name.clone(),
            FieldSchema::new(FieldType::Array).required(true),
        );
        self.required.push(name);
        self
    }

    /// Add a required object field
    pub fn object(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.fields.insert(
            name.clone(),
            FieldSchema::new(FieldType::Object).required(true),
        );
        self.required.push(name);
        self
    }

    /// Add a custom field schema
    pub fn field(mut self, name: impl Into<String>, schema: FieldSchema) -> Self {
        let name = name.into();
        if schema.is_required() {
            self.required.push(name.clone());
        }
        self.fields.insert(name, schema);
        self
    }

    /// Allow extra fields not defined in schema
    pub fn allow_extra_fields(mut self, allow: bool) -> Self {
        self.allow_extra = allow;
        self
    }

    /// Build the validation schema
    pub fn build(self) -> ValidationSchema {
        ValidationSchema {
            fields: self.fields,
            required: self.required,
            allow_extra_fields: self.allow_extra,
        }
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_builder_basic() {
        let schema = SchemaBuilder::new()
            .string("name")
            .integer("age")
            .optional_string("nickname")
            .build();

        assert_eq!(schema.fields.len(), 3);
        assert_eq!(schema.required.len(), 2);
    }

    #[test]
    fn test_quick_validate() {
        let schema = SchemaBuilder::new().string("name").integer("age").build();

        let valid_input = serde_json::json!({
            "name": "Alice",
            "age": 30
        });

        let result = validate(&valid_input, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quick_validate_missing_field() {
        let schema = SchemaBuilder::new().string("name").integer("age").build();

        let invalid_input = serde_json::json!({
            "name": "Alice"
        });

        let result = validate(&invalid_input, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_quick_validate_wrong_type() {
        let schema = SchemaBuilder::new().integer("age").build();

        let invalid_input = serde_json::json!({
            "age": "not a number"
        });

        let result = validate(&invalid_input, &schema);
        assert!(result.is_err());
    }
}
