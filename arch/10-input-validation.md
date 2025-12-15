# Input Validation Framework

## Overview

The validation module provides comprehensive input validation for tool arguments, including type checking, format validation, range constraints, and input sanitization.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Validation System                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌───────────────────────────────────────────────────┐     │
│  │              SchemaBuilder                         │     │
│  │  .string("name")                                   │     │
│  │  .integer("age")                                   │     │
│  │  .field("email", schema)                          │     │
│  │  .build()                                         │     │
│  └───────────────────────┬───────────────────────────┘     │
│                          │                                  │
│                          ▼                                  │
│  ┌───────────────────────────────────────────────────┐     │
│  │            ValidationSchema                        │     │
│  │  ┌─────────────────┐  ┌───────────────────────┐  │     │
│  │  │   FieldSchema   │  │   ValidationRule[]    │  │     │
│  │  │  - field_type   │  │  - MinLength          │  │     │
│  │  │  - required     │  │  - MaxLength          │  │     │
│  │  │  - default      │  │  - Pattern            │  │     │
│  │  │  - enum_values  │  │  - Email, URL         │  │     │
│  │  └─────────────────┘  └───────────────────────┘  │     │
│  └───────────────────────┬───────────────────────────┘     │
│                          │                                  │
│                          ▼                                  │
│  ┌───────────────────────────────────────────────────┐     │
│  │               Validator                            │     │
│  │  - Type checking                                   │     │
│  │  - Required fields                                 │     │
│  │  - Rule validation                                 │     │
│  │  - Nested validation                               │     │
│  │  - Array item validation                           │     │
│  └───────────────────────┬───────────────────────────┘     │
│                          │                                  │
│              ┌───────────┴───────────┐                     │
│              ▼                       ▼                      │
│       Ok(())               Err(ValidationError)            │
│                            ┌───────────────────┐           │
│                            │   FieldError[]    │           │
│                            │  - code           │           │
│                            │  - message        │           │
│                            │  - expected       │           │
│                            │  - actual         │           │
│                            └───────────────────┘           │
│                                                              │
│  ┌───────────────────────────────────────────────────┐     │
│  │           InputSanitizer                           │     │
│  │  - HTML removal                                    │     │
│  │  - Control char removal                            │     │
│  │  - Dangerous key filtering                         │     │
│  │  - Length truncation                               │     │
│  │  - Depth limiting                                  │     │
│  └───────────────────────────────────────────────────┘     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Components

### FieldType

```rust
pub enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
    Any,
    Null,
    Union(Vec<FieldType>),
}
```

### FieldSchema

```rust
pub struct FieldSchema {
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<Value>,
    pub description: Option<String>,
    pub rules: Vec<ValidationRule>,
    pub nested_schema: Option<Box<ValidationSchema>>,
    pub item_schema: Option<Box<FieldSchema>>,
    pub enum_values: Option<Vec<String>>,
}
```

### ValidationRule

```rust
pub enum ValidationRule {
    MinLength(usize),
    MaxLength(usize),
    MinValue(f64),
    MaxValue(f64),
    Pattern(String),
    Email,
    Url,
    Path,
    NonEmpty,
    MinItems(usize),
    MaxItems(usize),
    UniqueItems,
    Custom { name: String, message: String },
}
```

## Usage Examples

### Basic Schema

```rust
use sage_core::{SchemaBuilder, validate};

let schema = SchemaBuilder::new()
    .string("name")
    .integer("age")
    .optional_string("email")
    .build();

let input = serde_json::json!({
    "name": "Alice",
    "age": 30
});

match validate(&input, &schema) {
    Ok(()) => println!("Valid input"),
    Err(errors) => println!("Errors: {}", errors),
}
```

### Field Constraints

```rust
use sage_core::{FieldSchema, FieldType, ValidationSchema};

let mut schema = ValidationSchema::new();
schema.add_field(
    "username",
    FieldSchema::new(FieldType::String)
        .required(true)
        .min_length(3)
        .max_length(20)
        .pattern(r"^[a-zA-Z0-9_]+$")
);

schema.add_field(
    "score",
    FieldSchema::new(FieldType::Integer)
        .required(true)
        .min_value(0.0)
        .max_value(100.0)
);
```

### Enum Values

```rust
let field = FieldSchema::new(FieldType::String)
    .required(true)
    .enum_of(vec!["active", "inactive", "pending"]);
```

### Nested Objects

```rust
let address_schema = SchemaBuilder::new()
    .string("street")
    .string("city")
    .string("country")
    .build();

let user_schema = SchemaBuilder::new()
    .string("name")
    .field(
        "address",
        FieldSchema::new(FieldType::Object)
            .required(true)
            .nested(address_schema)
    )
    .build();
```

### Array Items

```rust
let schema = ValidationSchema::new();
schema.add_field(
    "scores",
    FieldSchema::new(FieldType::Array)
        .required(true)
        .rule(ValidationRule::MinItems(1))
        .rule(ValidationRule::MaxItems(10))
        .items(
            FieldSchema::new(FieldType::Integer)
                .min_value(0.0)
                .max_value(100.0)
        )
);
```

## Common Rules

### RuleSet

```rust
// Pre-built rule combinations
let rules = RuleSet::string_length(1, 100);
let rules = RuleSet::number_range(0.0, 100.0);
let rules = RuleSet::positive_number();
let rules = RuleSet::email();
let rules = RuleSet::url();
let rules = RuleSet::safe_path();
```

### CommonRules

```rust
// Pre-built field schemas
let path_field = CommonRules::file_path();
let command_field = CommonRules::command();
let positive_int = CommonRules::positive_integer();
let timeout = CommonRules::timeout_seconds();
let optional_bool = CommonRules::optional_boolean(false);
```

## Input Sanitization

### SanitizeOptions

```rust
pub struct SanitizeOptions {
    pub remove_nulls: bool,
    pub trim_strings: bool,
    pub max_string_length: Option<usize>,
    pub max_array_length: Option<usize>,
    pub max_depth: Option<usize>,
    pub remove_html: bool,
    pub remove_control_chars: bool,
    pub normalize_unicode: bool,
    pub remove_empty_strings: bool,
    pub remove_dangerous_keys: bool,
}
```

### Usage

```rust
use sage_core::{InputSanitizer, SanitizeOptions, sanitize};

// Quick sanitization
let clean = sanitize(&input, &SanitizeOptions::default());

// Strict sanitization
let clean = sanitize(&input, &SanitizeOptions::strict());

// Custom options
let options = SanitizeOptions {
    remove_html: true,
    max_string_length: Some(1000),
    remove_dangerous_keys: true,
    ..Default::default()
};
let sanitizer = InputSanitizer::new(options);
let clean = sanitizer.sanitize(&input);
```

### Security Features

**HTML Removal:**
```rust
// Input: "<script>alert('xss')</script>Hello"
// Output: "alert('xss')Hello"
```

**Dangerous Keys:**
```rust
// Removed keys: __proto__, constructor, prototype
// Protects against prototype pollution
```

**Control Characters:**
```rust
// Removes non-printable characters except \n, \r, \t
```

**Depth Limiting:**
```rust
// Prevents deeply nested structures (DoS protection)
```

## Error Handling

### ValidationError

```rust
pub struct ValidationError {
    pub field_errors: HashMap<String, Vec<FieldError>>,
    pub general_errors: Vec<String>,
}

impl ValidationError {
    pub fn has_errors(&self) -> bool;
    pub fn error_count(&self) -> usize;
    pub fn all_errors(&self) -> Vec<String>;
}
```

### FieldError

```rust
pub struct FieldError {
    pub code: String,      // e.g., "required", "type_mismatch"
    pub message: String,   // Human-readable message
    pub expected: Option<String>,
    pub actual: Option<String>,
}

// Common error constructors
FieldError::required()
FieldError::type_mismatch("integer", "string")
FieldError::invalid_format("email")
FieldError::out_of_range("Value must be between 0 and 100")
FieldError::invalid_enum(&["a", "b", "c"])
FieldError::unknown_field()
```

## Validator Configuration

```rust
let validator = Validator::new()
    .collect_all(true)  // Collect all errors (vs. stop at first)
    .custom("custom_rule", |value| {
        // Custom validation logic
        if value.as_str() == Some("forbidden") {
            Err("Value is forbidden".to_string())
        } else {
            Ok(())
        }
    });

let result = validator.validate(&input, &schema);
```

## Integration with Tools

Tools can define validation schemas for their inputs:

```rust
impl Tool for ReadFileTool {
    fn input_schema(&self) -> ValidationSchema {
        SchemaBuilder::new()
            .field("file_path", CommonRules::file_path())
            .optional_integer("offset")
            .optional_integer("limit")
            .build()
    }

    async fn execute(&self, args: Value) -> ToolResult {
        // Validation happens before execute is called
        let path = args["file_path"].as_str().unwrap();
        // ...
    }
}
```

## Test Coverage

- 44 unit tests covering:
  - Field type matching
  - All validation rules
  - Schema building
  - Nested object validation
  - Array item validation
  - Enum validation
  - Error collection
  - Input sanitization (HTML, control chars, dangerous keys)
  - Depth and length limiting
