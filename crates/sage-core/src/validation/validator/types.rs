//! Validation types and errors

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
        Self::new(
            "type_mismatch",
            format!("Expected {}, got {}", expected, actual),
        )
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
