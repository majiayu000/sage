# API Design Analysis

**Generated:** 2026-01-21
**Codebase:** Sage Agent

## Summary

This report analyzes the public API surface and identifies design improvements.

---

## 1. API Surface Overview

### Statistics
- **Total Public Items:** ~3,990 (pub fn/struct/enum/trait)
- **Source Files:** 968 Rust files
- **Crates:** 4 main crates (sage-core, sage-cli, sage-sdk, sage-tools)

### Crate Purposes
| Crate | Purpose | Public API |
|-------|---------|------------|
| sage-core | Core library | Internal + SDK use |
| sage-cli | CLI binary | Minimal |
| sage-sdk | Integration SDK | Primary public API |
| sage-tools | Tool implementations | Internal |

---

## 2. SDK API Design (sage-sdk)

### Current Structure
```
sage-sdk/src/
├── client/
│   ├── builder/
│   │   └── constructors.rs
│   └── mod.rs
└── lib.rs
```

### Good Patterns
1. Builder pattern for client construction
2. `validate_config()` for pre-flight checks
3. Clear separation from internal crates

### Improvement Opportunities
1. More comprehensive builder options
2. Async initialization pattern
3. Configuration presets

---

## 3. Tool API Design (sage-tools)

### Current Tool Trait
```rust
trait Tool {
    fn name(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    async fn execute(&self, call: ToolCall) -> ToolResult;
    fn validate(&self, call: &ToolCall) -> Result<(), ToolError>;
}
```

### Issues
1. **Boilerplate Heavy**: Every tool repeats similar code
2. **No Derive Macro**: Manual implementation required
3. **Inconsistent Validation**: Some tools skip validation

### Recommendations
```rust
// Derive macro approach
#[derive(Tool)]
#[tool(name = "read", description = "Reads a file")]
struct ReadTool;

impl ReadTool {
    async fn execute(&self, call: ToolCall) -> ToolResult {
        // Only business logic needed
    }
}
```

---

## 4. Error API Design

### Current Error Types
```rust
// sage-core
pub enum SageError { ... }
pub enum ToolError { ... }
pub enum LifecycleError { ... }

// sage-tools
// Uses anyhow::Error (inconsistent)
```

### Issues
1. Mixed typed errors and `anyhow` usage
2. Inconsistent error context
3. User-facing vs internal error separation needs work

### Recommendations
1. Standardize on typed errors in public APIs
2. Use `anyhow` only for internal implementation
3. Create clear error categories

---

## 5. Configuration API

### Current Design
```rust
pub struct AgentConfig {
    pub llm_config: LlmConfig,
    pub tools: ToolsConfig,
    // ... many fields
}
```

### Good Patterns
- Environment variable substitution
- JSON configuration format
- Validation on load

### Issues
1. Large configuration structs
2. Some optional fields without defaults
3. Configuration loading duplication (see duplication report)

### Recommendations
1. Builder pattern for configuration
2. Sensible defaults for all optional fields
3. Configuration profiles (dev, prod, test)

---

## 6. Async API Patterns

### Current State
- Tokio-based async runtime
- `async fn` throughout
- `async-trait` for trait objects

### Good Patterns
```rust
// Proper async streaming
pub async fn stream_response(&self) -> impl Stream<Item = Response>
```

### Issues
1. Some synchronous file I/O mixed with async
2. Lock usage needs async-aware patterns

### Recommendations
1. Use `tokio::fs` consistently
2. Switch to `tokio::sync::RwLock`
3. Document async cancellation safety

---

## 7. Type Safety Improvements

### Newtype Opportunities
```rust
// Current: stringly-typed
fn execute(&self, tool_name: String, params: String)

// Better: type-safe
fn execute(&self, tool: ToolName, params: ToolParams)
```

### Locations for Improvement
- Tool names (String → ToolName)
- File paths (String → ValidatedPath)
- Model names (String → ModelId)

### Benefits
1. Compile-time validation
2. Better documentation
3. Prevents mixing similar types

---

## 8. API Versioning

### Current State
- Workspace version: 0.5.1
- No explicit API versioning strategy
- Breaking changes possible

### Recommendations
1. Follow semantic versioning strictly
2. Mark experimental APIs clearly
3. Deprecation warnings before removal
4. Changelog maintenance

---

## 9. Documentation Coverage

### Statistics
- Files with module docs (`//!`): 960
- That's excellent coverage!

### Missing Documentation
- Some public types lack rustdoc
- Examples in doc comments limited
- Integration examples need expansion

### Recommendations
1. Add `#[deny(missing_docs)]` for public APIs
2. Include usage examples in rustdoc
3. Generate API reference documentation

---

## API Design Checklist

| Aspect | Status | Priority |
|--------|--------|----------|
| SDK Builder Pattern | Good | - |
| Tool Trait Design | Needs Work | P1 |
| Error Consistency | Needs Work | P1 |
| Configuration API | Good | P2 |
| Type Safety | Could Improve | P2 |
| Documentation | Good | P3 |
| Versioning Strategy | Needs Definition | P3 |

---

## Recommended Actions

### Phase 1: Core Improvements
1. Create Tool derive macro to reduce boilerplate
2. Standardize error types across crates
3. Add newtype wrappers for common types

### Phase 2: Polish
4. Configuration builder pattern
5. API documentation audit
6. Add `#[deny(missing_docs)]`

### Phase 3: Stability
7. Define versioning strategy
8. Mark stable vs experimental APIs
9. Create migration guides for breaking changes
