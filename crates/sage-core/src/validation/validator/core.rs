//! Core validator implementation

use super::types::{FieldError, ValidationError, ValidationResult};
use crate::validation::schema::{FieldSchema, ValidationSchema};
use serde_json::Value;
use std::collections::HashMap;

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
                errors.add_field_error(field_name, FieldError::new(rule.rule_name(), msg));
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
