# Error Handling Pattern Analysis Report

## Summary

Analyzed `/Users/lifcc/Desktop/code/AI/agent/sage/crates/sage-core/src/error.rs`

- **Total public functions**: 23
- **Lines of code**: 635 lines
- **Error variants**: 13 variants

## Pattern Classification

### 1. Basic Constructors (12 functions)

Simple constructors that only take a `message` parameter:

```rust
pub fn xxx(message: impl Into<String>) -> Self {
    Self::Xxx {
        message: message.into(),
        context: None,
        // variant-specific fields set to None/default
    }
}
```

**Functions following this pattern:**
- `config(message)` → Config variant
- `llm(message)` → Llm variant
- `agent(message)` → Agent variant
- `cache(message)` → Cache variant
- `invalid_input(message)` → InvalidInput variant
- `storage(message)` → Storage variant
- `not_found(message)` → NotFound variant
- `execution(message)` → Agent variant (alias)
- `io(message)` → Io variant
- `json(message)` → Json variant
- `http(message)` → Http variant
- `other(message)` → Other variant

**Code repetition**: ~120 lines (10 lines × 12 functions)

### 2. With-Context Constructors (5 functions)

Constructors that take `message` and `context` parameters:

```rust
pub fn xxx_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
    Self::Xxx {
        message: message.into(),
        context: Some(context.into()),
        // variant-specific fields
    }
}
```

**Functions following this pattern:**
- `config_with_context(message, context)`
- `agent_with_context(message, context)`
- `tool_with_context(tool_name, message, context)` (also has tool_name)

**Code repetition**: ~45 lines (9 lines × 5 functions)

### 3. With-Field Constructors (5 functions)

Constructors that take `message` plus one variant-specific field:

```rust
pub fn xxx_with_yyy(message: impl Into<String>, yyy: YyyType) -> Self {
    Self::Xxx {
        message: message.into(),
        yyy: Some(yyy.into()),
        context: None,
    }
}
```

**Functions following this pattern:**
- `llm_with_provider(message, provider)`
- `invalid_input_field(message, field)`
- `not_found_resource(message, resource_type)`
- `io_with_path(message, path)`
- `http_with_status(message, status_code)`

**Code repetition**: ~50 lines (10 lines × 5 functions)

### 4. Special Cases (2 functions)

- `tool(tool_name, message)` - Requires 2 parameters, no simpler version
- `timeout(seconds)` - Only takes `seconds`, no message parameter
- `with_context(self, context)` - Chainable method

## Repetition Statistics

| Pattern Type | Count | Lines per Function | Total Repetition |
|--------------|-------|-------------------|------------------|
| Basic constructors | 12 | ~10 | ~120 lines |
| With-context constructors | 5 | ~9 | ~45 lines |
| With-field constructors | 5 | ~10 | ~50 lines |
| **Total** | **22** | - | **~215 lines** |

**Percentage of file dedicated to constructors**: ~34% (215/635)

## Identified Issues

1. **High Boilerplate**: Every error variant requires 1-3 constructor functions
2. **Maintenance Burden**: Adding a new error variant requires writing multiple similar functions
3. **Inconsistent API**: Some variants have `_with_xxx` variants, others don't
4. **No Type Safety**: Context and other optional fields can't be enforced at compile time

## Recommended Improvements

### Option 1: Builder Pattern (Recommended)

**Pros:**
- Single implementation for all error types
- Compile-time safety for required fields
- Extensible without adding new functions
- ~200 lines reduction

**Example:**
```rust
// Instead of 3 functions:
SageError::config("msg")
SageError::config_with_context("msg", "ctx")
SageError::Config { message, source, context }

// Use builder:
SageError::builder(ErrorKind::Config, "msg")
    .context("ctx")
    .source(err)
    .build()
```

**Implementation:**
```rust
pub struct SageErrorBuilder {
    kind: ErrorKind,
    message: String,
    context: Option<String>,
    // variant-specific fields
    provider: Option<String>,
    tool_name: Option<String>,
    // ...
}

impl SageErrorBuilder {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self { ... }
    pub fn context(mut self, ctx: impl Into<String>) -> Self { ... }
    pub fn provider(mut self, p: impl Into<String>) -> Self { ... }
    pub fn build(self) -> SageError { ... }
}

impl SageError {
    pub fn builder(kind: ErrorKind, msg: impl Into<String>) -> SageErrorBuilder {
        SageErrorBuilder::new(kind, msg)
    }
}
```

### Option 2: Macro-Based Generation

**Pros:**
- Keeps existing API
- Reduces code duplication
- No breaking changes

**Cons:**
- Macros harder to debug
- Still verbose at call sites

**Example:**
```rust
macro_rules! impl_error_constructor {
    ($name:ident, $variant:ident, $($field:ident: $ty:ty),*) => {
        pub fn $name(message: impl Into<String> $(, $field: $ty)*) -> Self {
            Self::$variant {
                message: message.into(),
                $($field: Some($field.into()),)*
                context: None,
            }
        }
    };
}
```

### Option 3: Keep Current + Add Builder

**Pros:**
- Backward compatible
- Gradual migration
- Provides both simple and advanced APIs

**Cons:**
- More API surface area
- Duplicated functionality

## Recommendation

**Implement Option 1 (Builder Pattern)** for the following reasons:

1. **Reduces ~200 lines of boilerplate** (34% file size reduction)
2. **More flexible** - can add new optional fields without new functions
3. **Better ergonomics** - method chaining is intuitive
4. **Type-safe** - builder can enforce required fields at compile time
5. **Future-proof** - easy to extend without API changes

### Migration Path

1. Add `ErrorKind` enum (if not exists)
2. Implement `SageErrorBuilder`
3. Keep existing constructors for backward compatibility
4. Gradually migrate codebase to use builder
5. Mark old constructors as `#[deprecated]` after migration (following project rules, these would be removed after version bump)

## Appendix: All Constructor Functions

```
Line 196: config(message)
Line 205: config_with_context(message, context)
Line 214: llm(message)
Line 223: llm_with_provider(message, provider)
Line 232: tool(tool_name, message)
Line 241: tool_with_context(tool_name, message, context)
Line 254: agent(message)
Line 262: agent_with_context(message, context)
Line 270: cache(message)
Line 278: invalid_input(message)
Line 287: invalid_input_field(message, field)
Line 296: timeout(seconds)
Line 304: storage(message)
Line 312: not_found(message)
Line 321: not_found_resource(message, resource_type)
Line 333: execution(message)
Line 341: io(message)
Line 350: io_with_path(message, path)
Line 359: json(message)
Line 367: http(message)
Line 377: http_with_status(message, status_code)
Line 387: other(message)
Line 395: with_context(self, context)
```
