# Code Duplication Analysis

**Generated:** 2026-01-21
**Codebase:** Sage Agent

## Summary

This report identifies duplicated code patterns that could be consolidated for better maintainability.

---

## 1. Parameter Extraction Pattern (High Duplication)

### Issue
Repetitive `call.get_string("param").ok_or_else()` pattern across all tools.

### Example (Appears 50+ times)
```rust
let file_path = call.get_string("file_path").ok_or_else(|| {
    ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
})?;
```

### Locations
- `sage-tools/src/tools/file_ops/write/schema.rs`
- `sage-tools/src/tools/file_ops/edit.rs`
- `sage-tools/src/tools/file_ops/read/tool.rs`
- `sage-tools/src/tools/file_ops/grep/schema.rs`
- `sage-tools/src/tools/file_ops/glob/schema.rs`
- `sage-tools/src/tools/file_ops/notebook_edit/mod.rs`
- And 20+ more files

### Recommendation
Create a helper macro or trait method:
```rust
trait ToolCallExt {
    fn require_string(&self, param: &str) -> Result<String, ToolError>;
    fn require_path(&self, param: &str) -> Result<PathBuf, ToolError>;
}
```

---

## 2. Tool Trait Implementation Boilerplate (High Duplication)

### Issue
Every tool implements the same boilerplate for `Tool` trait.

### Pattern
```rust
fn name(&self) -> &str { "tool_name" }
fn schema(&self) -> ToolSchema { ... }
async fn execute(&self, call: ToolCall) -> ToolResult { ... }
async fn validate(&self, call: &ToolCall) -> Result<(), ToolError> { ... }
```

### Locations
All 40+ tool implementations in `sage-tools/src/tools/`

### Recommendation
Consider a derive macro:
```rust
#[derive(Tool)]
#[tool(name = "read", description = "Reads a file")]
struct ReadTool;
```

---

## 3. Error Message Patterns (Medium Duplication)

### Issue
Similar error message formatting across the codebase.

### Examples
```rust
// Pattern 1: anyhow! with format
Err(anyhow!("Failed to {}: {}", action, error))

// Pattern 2: context chaining
.context("Failed to X")?
.context("Failed to Y")?
```

### Locations
- `sage-tools/src/tools/network/validation.rs`
- `sage-tools/src/tools/container/docker/commands.rs`
- `sage-core/src/agent/unified/*.rs`

### Recommendation
Create an error builder or macro for consistent formatting.

---

## 4. URL Validation Logic (Medium Duplication)

### Issue
Nearly identical URL validation code in multiple places.

### Locations
- `sage-tools/src/tools/network/validation.rs`
- `sage-tools/src/tools/network/http_client/validation.rs`

### Overlapping Checks
- Scheme validation (http/https)
- Host extraction
- Blocked host checking
- Private IP detection

### Recommendation
Extract to shared `UrlValidator` struct in a common module.

---

## 5. Configuration Loading (Medium Duplication)

### Issue
Similar configuration loading patterns in multiple entry points.

### Locations
- `sage-cli/src/commands/unified/execute.rs`
- `sage-cli/src/ui/rnk_app.rs`
- `sage-sdk/src/client/builder/`

### Recommendation
Create a unified `ConfigLoader` that handles:
- File discovery
- Environment variable substitution
- Validation
- Defaults

---

## 6. Git Command Execution (Low Duplication)

### Issue
Similar patterns for executing git commands.

### Locations
- `sage-tools/src/tools/vcs/git/executor.rs`
- `sage-tools/src/tools/vcs/git/operations/*.rs`

### Recommendation
Create a `GitCommand` builder with common error handling.

---

## 7. Database Connection Patterns (Low Duplication)

### Issue
Similar connection string validation and parsing.

### Locations
- `sage-tools/src/tools/database/sql/validation.rs`
- `sage-tools/src/tools/database/mongodb.rs`

### Recommendation
Create a `DatabaseConnectionValidator` trait.

---

## Duplication Metrics

| Category | Files Affected | Lines Duplicated | Priority |
|----------|----------------|------------------|----------|
| Parameter Extraction | 25+ | ~400 | P1 |
| Tool Trait Boilerplate | 40+ | ~600 | P1 |
| Error Messages | 30+ | ~200 | P2 |
| URL Validation | 2 | ~100 | P2 |
| Config Loading | 3 | ~80 | P3 |
| Git Commands | 5 | ~60 | P3 |
| DB Connections | 3 | ~40 | P3 |

---

## Refactoring Plan

### Phase 1: High Impact
1. Create `ToolCallExt` trait for parameter extraction
2. Design tool derive macro for boilerplate reduction

### Phase 2: Medium Impact
3. Consolidate URL validation into shared module
4. Create error message helpers/macros

### Phase 3: Low Impact
5. Unify configuration loading
6. Extract Git command patterns
7. Database connection abstraction
