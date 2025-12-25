//! Tests for validator

#[cfg(test)]
mod tests {
    use super::super::{FieldError, ValidationError, Validator};
    use crate::validation::SchemaBuilder;
    use crate::validation::schema::{FieldSchema, FieldType, ValidationSchema};

    #[test]
    fn test_validate_required_fields() {
        let schema = SchemaBuilder::new().string("name").integer("age").build();

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
        let schema = SchemaBuilder::new().integer("count").build();

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
        let schema = SchemaBuilder::new().string("name").build();

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
        let address_schema = SchemaBuilder::new().string("street").string("city").build();

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
        let mut schema = ValidationSchema::new();
        schema.add_field(
            "scores",
            FieldSchema::new(FieldType::Array).required(true).items(
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
