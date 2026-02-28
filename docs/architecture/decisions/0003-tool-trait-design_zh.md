# ADR-0003: Tool Trait Design

## Status

Accepted

## Context

Sage Agent's core capability is executing tools on behalf of the LLM. We needed a design that:

1. **Enables extensibility**: Users can add custom tools
2. **Ensures type safety**: Tool arguments and results are validated
3. **Supports async execution**: Tools perform I/O operations
4. **Provides fine-grained control**:
   - Permission checking
   - Timeout management
   - Concurrency control
   - Risk assessment
5. **Integrates with LLMs**: Tools must expose schemas for function calling
6. **Handles diverse use cases**: File operations, bash commands, web requests, planning, etc.

Constraints:
- Tools range from low-risk (read file) to high-risk (execute bash)
- Some tools must run sequentially, others can run in parallel
- Need user approval workflows for dangerous operations
- Must support both streaming and one-shot execution contexts

## Decision

We designed a **trait-based tool system** with rich metadata and lifecycle hooks.

### Core Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    // Identity
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    // Schema for LLM function calling
    fn schema(&self) -> ToolSchema;

    // Execution lifecycle
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;
    fn validate(&self, call: &ToolCall) -> Result<(), ToolError>;

    // Permission and safety
    async fn check_permission(&self, call: &ToolCall, context: &ToolContext)
        -> PermissionResult;
    fn risk_level(&self) -> RiskLevel;

    // Execution control
    fn concurrency_mode(&self) -> ConcurrencyMode;
    fn max_execution_duration(&self) -> Option<Duration>;
    fn is_read_only(&self) -> bool;

    // User interaction
    fn requires_user_interaction(&self) -> bool;
    fn render_call(&self, call: &ToolCall) -> String;
    fn render_result(&self, result: &ToolResult) -> String;
}
```

### Key Design Elements

1. **Concurrency Control**:
   ```rust
   pub enum ConcurrencyMode {
       Parallel,                // Any number can run concurrently
       Sequential,              // Global sequential execution
       Limited(usize),          // Max N concurrent instances
       ExclusiveByType,         // Only one per tool type
   }
   ```

2. **Permission System**:
   ```rust
   pub enum PermissionResult {
       Allow,                   // Execute immediately
       Deny(String),            // Block with reason
       RequireApproval(String), // Ask user first
   }

   pub enum RiskLevel {
       Low,    // Read-only operations
       Medium, // Writes, network requests
       High,   // Bash execution, destructive ops
   }
   ```

3. **Validation Pipeline**:
   ```rust
   async fn execute_with_timing(&self, call: &ToolCall) -> ToolResult {
       // 1. Validate arguments
       if let Err(err) = self.validate(call) {
           return ToolResult::error(...);
       }

       // 2. Execute tool
       let result = self.execute(call).await;

       // 3. Track timing
       result.with_execution_time(elapsed)
   }
   ```

4. **Helper Traits**:
   ```rust
   // For file system tools
   pub trait FileSystemTool: Tool {
       fn working_directory(&self) -> &Path;
       fn resolve_path(&self, path: &str) -> PathBuf;
       fn is_safe_path(&self, path: &Path) -> bool; // Sandbox validation
   }

   // For command execution tools
   pub trait CommandTool: Tool {
       fn allowed_commands(&self) -> Vec<&str>;
       fn is_command_allowed(&self, command: &str) -> bool;
       fn command_working_directory(&self) -> &Path;
   }
   ```

### Tool Registration

Tools are registered in a `ToolRegistry`:
```rust
let mut registry = ToolRegistry::new();
registry.register(Box::new(ReadFileTool::new(work_dir.clone())));
registry.register(Box::new(WriteFileTool::new(work_dir.clone())));
registry.register(Box::new(BashTool::new(work_dir.clone())));
```

### Execution Flow

1. **LLM requests tool call** → `ToolCall` with arguments
2. **Permission check** → `check_permission()` with context
3. **Validation** → `validate()` checks argument schema
4. **Execution** → `execute()` performs operation (with timeout)
5. **Result rendering** → `render_result()` formats output
6. **Return to LLM** → `ToolResult` included in next message

## Consequences

### Positive

1. **Type Safety**:
   - `ToolSchema` ensures LLM provides correct arguments
   - Rust's type system prevents runtime errors
   - `serde_json` handles serialization/deserialization

2. **Extensibility**:
   - Users implement the `Tool` trait for custom tools
   - No modification to core agent required
   - Macro `impl_tool!` reduces boilerplate

3. **Security**:
   - `FileSystemTool::is_safe_path()` prevents path traversal
   - `check_permission()` enables user approval workflows
   - `RiskLevel` guides when to ask for confirmation
   - Sandbox working directory isolation

4. **Concurrency Control**:
   - `ConcurrencyMode::Sequential` for tools that modify shared state
   - `ConcurrencyMode::Parallel` for I/O-bound operations
   - Prevents race conditions and resource exhaustion

5. **Observability**:
   - `execute_with_timing()` tracks performance
   - `render_call()` and `render_result()` provide user visibility
   - Structured `ToolResult` enables analytics

6. **Async-First**:
   - All execution is async (`#[async_trait]`)
   - Integrates with Tokio runtime
   - Supports timeouts via `tokio::time::timeout`

7. **User Experience**:
   - `requires_user_interaction()` for tools like `ask_user_question`
   - Rich rendering of tool calls and results
   - Clear error messages via `ToolError`

### Negative

1. **Trait Object Overhead**:
   - `Box<dyn Tool>` has dynamic dispatch cost
   - Acceptable for I/O-bound operations
   - Could use generics for zero-cost abstraction (rejected for simplicity)

2. **Async Trait Complexity**:
   - Requires `#[async_trait]` macro
   - Adds some compile-time overhead
   - Boxing futures has minor runtime cost

3. **Schema Definition**:
   - Tools must define JSON schemas
   - Duplicates type information
   - Could use derive macros (future improvement)

4. **Validation Duplication**:
   - Schema validation in LLM client
   - Additional validation in `validate()`
   - Necessary for defense-in-depth

### Design Patterns

1. **Template Method Pattern**:
   - `execute_with_timing()` defines execution skeleton
   - Tools implement `execute()` for specific behavior

2. **Strategy Pattern**:
   - Different permission strategies via `check_permission()`
   - Different concurrency strategies via `concurrency_mode()`

3. **Builder Pattern** (for tools):
   ```rust
   BashTool::builder()
       .work_dir(path)
       .allowed_commands(vec!["git", "cargo"])
       .build()
   ```

### Alternative Approaches Considered

1. **Enum-Based Tools**:
   ```rust
   enum Tool {
       ReadFile { path: String },
       Bash { command: String },
   }
   ```
   - Rejected: Not extensible, hard to add custom tools
   - All tools must be defined in core

2. **Function-Based Tools**:
   ```rust
   type ToolFn = Box<dyn Fn(ToolCall) -> ToolResult>;
   ```
   - Rejected: Lacks metadata (schema, permissions, concurrency)
   - No way to query tool capabilities

3. **Proc Macro Derive**:
   ```rust
   #[derive(Tool)]
   struct MyTool { ... }
   ```
   - Future consideration
   - Requires complex macro implementation
   - Current trait approach is explicit and flexible

4. **Separate Validation Trait**:
   ```rust
   trait Validator {
       fn validate(&self, call: &ToolCall) -> Result<()>;
   }
   ```
   - Rejected: Splits related functionality
   - Current design keeps validation with execution

## Migration Path

For adding new tools:
1. Implement `Tool` trait
2. Optionally implement `FileSystemTool` or `CommandTool`
3. Register in `ToolRegistry`
4. Define JSON schema for LLM integration

## Performance Considerations

- **Parallel Execution**: `ConcurrencyMode::Parallel` tools leverage Tokio's scheduler
- **Timeout Enforcement**: `max_execution_duration()` prevents hung operations
- **Caching**: Tool schemas are cached in memory (computed once per tool instance)

## References

- `crates/sage-core/src/tools/base.rs`: Tool trait definition
- `crates/sage-core/src/tools/registry.rs`: Tool registration
- `crates/sage-core/src/tools/executor.rs`: Execution engine
- `crates/sage-core/src/tools/permission/`: Permission system
- `crates/sage-tools/src/`: Built-in tool implementations
