//! Validator implementation

use super::schema::{FieldSchema, FieldType, ValidationSchema};
use serde_json::Value;
use std::collections::HashMap;

/// Validation result
pub type ValidationResult = Result<(), ValidationError>;

/// Validation error containing all field errors
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Errors by field name
    pub field_errors: HashMap<String, Vec<FieldError>>,

    /// General errors not related to specific fields
    pub general_errors: Vec<String>,
}

impl ValidationError {
    /// Create empty validation error
    pub fn new() -> Self {
        Self {
            field_errors: HashMap::new(),
            general_errors: Vec::new(),
        }
    }

    /// Add field error
    pub fn add_field_error(&mut self, field: impl Into<String>, error: FieldError) {
        self.field_errors
            .entry(field.into())
            .or_default()
            .push(error);
    }

    /// Add general error
    pub fn add_general_error(&mut self, error: impl Into<String>) {
        self.general_errors.push(error.into());
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.field_errors.is_empty() || !self.general_errors.is_empty()
    }

    /// Get total error count
    pub fn error_count(&self) -> usize {
        self.field_errors.values().map(|v| v.len()).sum::<usize>() + self.general_errors.len()
    }

    /// Get all errors as strings
    pub fn all_errors(&self) -> Vec<String> {
        let mut errors = self.general_errors.clone();
        for (field, field_errors) in &self.field_errors {
            for error in field_errors {
                errors.push(format!("{}: {}", field, error.message));
            }
        }
        errors
    }
}

impl Default for ValidationError {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let errors = self.all_errors();
        write!(f, "Validation failed: {}", errors.join("; "))
    }
}

impl std::error::Error for ValidationError {}

/// Error for a single field
#[derive(Debug, Clone)]
pub struct FieldError {
    /// Error code
    pub code: String,

    /// Human-readable message
    pub message: String,

    /// Expected value or format
    pub expected: Option<String>,

    /// Actual value received
    pub actual: Option<String>,
}

impl FieldError {
    /// Create new field error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    /// Add expected value
    pub fn expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    /// Add actual value
    pub fn actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }

    /// Create "required" error
    pub fn required() -> Self {
        Self::new("required", "Field is required")
    }

    /// Create "type_mismatch" error
    pub fn type_mismatch(expected: &str, actual: &str) -> Self {
        Self::new("type_mismatch", format!("Expected {}, got {}", expected, actual))
            .expected(expected)
            .actual(actual)
    }

    /// Create "invalid_format" error
    pub fn invalid_format(format: &str) -> Self {
        Self::new("invalid_format", format!("Invalid {} format", format))
    }

    /// Create "out_of_range" error
    pub fn out_of_range(message: impl Into<String>) -> Self {
        Self::new("out_of_range", message)
    }

    /// Create "invalid_enum" error
    pub fn invalid_enum(allowed: &[String]) -> Self {
        Self::new(
            "invalid_enum",
            format!("Value must be one of: {}", allowed.join(", ")),
        )
    }

    /// Create "unknown_field" error
    pub fn unknown_field() -> Self {
        Self::new("unknown_field", "Unknown field")
    }
}

/// Main validator struct
pub struct Validator {
    /// Whether to collect all errors or stop at first
    collect_all: bool,

    /// Custom validators
    custom_validators: HashMap<String, Box<dyn Fn(&Value) -> Result<(), String> + Send + Sync>>,
}

impl Validator {
    /// Create new validator
    pub fn new() -> Self {
        Self {
            collect_all: true,
            custom_validators: HashMap::new(),
        }
    }

    /// Set whether to collect all errors
    pub fn collect_all(mut self, collect: bool) -> Self {
        self.collect_all = collect;
        self
    }

    /// Add custom validator
    pub fn custom<F>(mut self, name: impl Into<String>, validator: F) -> Self
    where
        F: Fn(&Value) -> Result<(), String> + Send + Sync + 'static,
    {
        self.custom_validators
            .insert(name.into(), Box::new(validator));
        self
    }

    /// Validate a value against a schema
    pub fn validate(&self, value: &Value, schema: &ValidationSchema) -> ValidationResult {
        let mut errors = ValidationError::new();

        // Value must be an object
        let obj = match value.as_object() {
            Some(obj) => obj,
            None => {
                errors.add_general_error("Input must be an object");
                return Err(errors);
            }
        };

        // Check required fields
        for required_field in &schema.required {
            if !obj.contains_key(required_field) {
                errors.add_field_error(required_field, FieldError::required());
                if !self.collect_all {
                    return Err(errors);
                }
            }
        }

        // Validate each field
        for (field_name, field_value) in obj {
            if let Some(field_schema) = schema.fields.get(field_name) {
                self.validate_field(field_name, field_value, field_schema, &mut errors);
                if !self.collect_all && errors.has_errors() {
                    return Err(errors);
                }
            } else if !schema.allow_extra_fields {
                errors.add_field_error(field_name, FieldError::unknown_field());
                if !self.collect_all {
                    return Err(errors);
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    /// Validate a single field
    fn validate_field(
        &self,
        field_name: &str,
        value: &Value,
        schema: &FieldSchema,
        errors: &mut ValidationError,
    ) {
        // Type check
        if !schema.field_type.matches(value) {
            let actual_type = self.get_value_type(value);
            errors.add_field_error(
                field_name,
                FieldError::type_mismatch(schema.field_type.type_name(), &actual_type),
            );
            return; // Don't continue validation if type is wrong
        }

        // Enum check
        if let Some(ref enum_values) = schema.enum_values {
            if let Some(s) = value.as_str() {
                if !enum_values.contains(&s.to_string()) {
                    errors.add_field_error(field_name, FieldError::invalid_enum(enum_values));
                }
            }
        }

        // Apply validation rules
        for rule in &schema.rules {
            if let Err(msg) = rule.validate(value) {
                errors.add_field_error(
                    field_name,
                    FieldError::new(rule.rule_name(), msg),
                );
            }
        }

        // Nested object validation
        if let (Some(nested_schema), Some(obj)) = (&schema.nested_schema, value.as_object()) {
            let nested_value = Value::Object(obj.clone());
            if let Err(nested_errors) = self.validate(&nested_value, nested_schema) {
                for (nested_field, nested_field_errors) in nested_errors.field_errors {
                    let full_field = format!("{}.{}", field_name, nested_field);
                    for error in nested_field_errors {
                        errors.add_field_error(&full_field, error);
                    }
                }
            }
        }

        // Array item validation
        if let (Some(item_schema), Some(arr)) = (&schema.item_schema, value.as_array()) {
            for (index, item) in arr.iter().enumerate() {
                let item_field = format!("{}[{}]", field_name, index);
                self.validate_field(&item_field, item, item_schema, errors);
            }
        }
    }

    /// Get the type name of a JSON value
    fn get_value_type(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Number(n) if n.is_i64() || n.is_u64() => "integer".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(),
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::SchemaBuilder;

    #[test]
    fn test_validate_required_fields() {
        let schema = SchemaBuilder::new()
            .string("name")
            .integer("age")
            .build();

        let validator = Validator::new();

        // Valid input
        let valid = serde_json::json!({
            "name": "Alice",
            "age": 30
        });
        assert!(validator.validate(&valid, &schema).is_ok());

        // Missing required field
        let invalid = serde_json::json!({
            "name": "Alice"
        });
        let result = validator.validate(&invalid, &schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.field_errors.contains_key("age"));
    }

    #[test]
    fn test_validate_type_mismatch() {
        let schema = SchemaBuilder::new()
            .integer("count")
            .build();

        let validator = Validator::new();

        let invalid = serde_json::json!({
            "count": "not a number"
        });
        let result = validator.validate(&invalid, &schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.field_errors.contains_key("count"));
        assert_eq!(err.field_errors["count"][0].code, "type_mismatch");
    }

    #[test]
    fn test_validate_with_rules() {
        use super::super::schema::FieldSchema;

        let mut schema = ValidationSchema::new();
        schema.add_field(
            "username",
            FieldSchema::new(FieldType::String)
                .required(true)
                .min_length(3)
                .max_length(20),
        );

        let validator = Validator::new();

        // Valid
        let valid = serde_json::json!({ "username": "alice" });
        assert!(validator.validate(&valid, &schema).is_ok());

        // Too short
        let invalid = serde_json::json!({ "username": "ab" });
        assert!(validator.validate(&invalid, &schema).is_err());

        // Too long
        let invalid = serde_json::json!({ "username": "this_username_is_too_long_to_be_valid" });
        assert!(validator.validate(&invalid, &schema).is_err());
    }

    #[test]
    fn test_validate_enum() {
        use super::super::schema::FieldSchema;

        let mut schema = ValidationSchema::new();
        schema.add_field(
            "status",
            FieldSchema::new(FieldType::String)
                .required(true)
                .enum_of(vec!["active", "inactive", "pending"]),
        );

        let validator = Validator::new();

        // Valid
        let valid = serde_json::json!({ "status": "active" });
        assert!(validator.validate(&valid, &schema).is_ok());

        // Invalid enum value
        let invalid = serde_json::json!({ "status": "unknown" });
        let result = validator.validate(&invalid, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_unknown_fields() {
        let schema = SchemaBuilder::new()
            .string("name")
            .build();

        let validator = Validator::new();

        let with_extra = serde_json::json!({
            "name": "Alice",
            "extra": "field"
        });

        // By default, unknown fields are not allowed
        let result = validator.validate(&with_extra, &schema);
        assert!(result.is_err());

        // Allow extra fields
        let schema_with_extra = SchemaBuilder::new()
            .string("name")
            .allow_extra_fields(true)
            .build();

        let result = validator.validate(&with_extra, &schema_with_extra);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_validation() {
        use super::super::schema::FieldSchema;

        let address_schema = SchemaBuilder::new()
            .string("street")
            .string("city")
            .build();

        let mut schema = ValidationSchema::new();
        schema.add_field(
            "address",
            FieldSchema::new(FieldType::Object)
                .required(true)
                .nested(address_schema),
        );

        let validator = Validator::new();

        // Valid nested object
        let valid = serde_json::json!({
            "address": {
                "street": "123 Main St",
                "city": "Springfield"
            }
        });
        assert!(validator.validate(&valid, &schema).is_ok());

        // Invalid nested object (missing city)
        let invalid = serde_json::json!({
            "address": {
                "street": "123 Main St"
            }
        });
        let result = validator.validate(&invalid, &schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.field_errors.contains_key("address.city"));
    }

    #[test]
    fn test_array_item_validation() {
        use super::super::schema::FieldSchema;

        let mut schema = ValidationSchema::new();
        schema.add_field(
            "scores",
            FieldSchema::new(FieldType::Array)
                .required(true)
                .items(
                    FieldSchema::new(FieldType::Integer)
                        .min_value(0.0)
                        .max_value(100.0),
                ),
        );

        let validator = Validator::new();

        // Valid array
        let valid = serde_json::json!({
            "scores": [85, 90, 75]
        });
        assert!(validator.validate(&valid, &schema).is_ok());

        // Invalid item (out of range)
        let invalid = serde_json::json!({
            "scores": [85, 150, 75]
        });
        let result = validator.validate(&invalid, &schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.field_errors.contains_key("scores[1]"));
    }

    #[test]
    fn test_error_display() {
        let mut errors = ValidationError::new();
        errors.add_field_error("name", FieldError::required());
        errors.add_field_error("age", FieldError::type_mismatch("integer", "string"));

        let display = format!("{}", errors);
        assert!(display.contains("name"));
        assert!(display.contains("age"));
    }

    #[test]
    fn test_collect_all_errors() {
        let schema = SchemaBuilder::new()
            .string("name")
            .integer("age")
            .string("email")
            .build();

        // With collect_all = true (default)
        let validator = Validator::new();
        let invalid = serde_json::json!({});
        let result = validator.validate(&invalid, &schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_count(), 3); // All missing fields reported
    }
}
