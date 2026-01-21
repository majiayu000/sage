# Architecture Issues Analysis

**Generated:** 2026-01-21
**Codebase:** Sage Agent

## Summary

This report analyzes the overall architecture and identifies structural improvements.

---

## 1. Architecture Overview

### Crate Structure
```
sage/
├── crates/
│   ├── sage-core/     # Core library (agent, llm, tools, mcp, etc.)
│   ├── sage-cli/      # Command-line interface
│   ├── sage-sdk/      # Public SDK for integrations
│   └── sage-tools/    # Built-in tool implementations
└── Cargo.toml         # Workspace configuration
```

### Dependency Graph
```
sage-cli ──► sage-sdk ──► sage-core ◄── sage-tools
                              │
                              ▼
                        sage-tools
```

---

## 2. Good Architecture Decisions

### 1. Workspace Organization
- Clear separation of concerns
- SDK provides stable public API
- Tools isolated from core

### 2. Async-First Design
- Built on Tokio runtime
- Non-blocking I/O throughout
- Stream-based LLM responses

### 3. Plugin Architecture
- MCP protocol support
- Tool discovery mechanism
- Extensible tool system

### 4. Configuration System
- Multi-source configuration
- Environment variable substitution
- Validation on load

---

## 3. Architectural Issues

### Issue 1: sage-core Module Size

**Problem:** sage-core contains too many responsibilities.

**Current Structure:**
```
sage-core/src/
├── agent/          # Agent execution
├── llm/            # LLM client layer
├── tools/          # Tool system
├── mcp/            # MCP protocol
├── config/         # Configuration
├── session/        # Session management
├── cache/          # Caching
├── memory/         # Memory management
├── learning/       # Learning system
├── recovery/       # Error recovery
├── sandbox/        # Sandboxing
├── hooks/          # Lifecycle hooks
├── modes/          # Mode management
├── skills/         # Skill system
├── prompts/        # Prompt templates
├── context/        # Context management
├── output/         # Output strategies
├── input/          # Input handling
├── commands/       # Command system
├── interrupt/      # Interrupt handling
├── trajectory/     # Trajectory recording
├── settings/       # Settings
├── plugins/        # Plugin system
└── ...             # More modules
```

**Recommendation:**
Consider splitting into smaller crates:
```
sage-llm/        # LLM client layer
sage-agent/      # Agent execution logic
sage-mcp/        # MCP protocol implementation
sage-session/    # Session management
```

---

### Issue 2: Circular Dependency Potential

**Concern:** Close coupling between modules could lead to circular dependencies.

**Example:**
- `agent` depends on `tools`
- `tools` may need `agent` context

**Recommendation:**
1. Define clear interfaces between modules
2. Use trait objects for dependency injection
3. Invert dependencies where needed

---

### Issue 3: Feature Creep

**Observation:** Many optional features in sage-core:
- Learning system
- Recovery patterns
- Checkpoint system
- Skill system

**Recommendation:**
1. Make optional features conditionally compiled
2. Use cargo features: `default = ["core"]`
3. Allow users to enable: `--features "learning,recovery"`

---

### Issue 4: Test Organization

**Current:**
- Unit tests scattered in source
- Integration tests in `tests/` directory
- Some tests inline with implementation

**Recommendation:**
Standardize test organization:
```
crate/
├── src/
│   └── module/
│       ├── mod.rs
│       └── tests.rs    # Unit tests
└── tests/
    ├── integration/    # Integration tests
    └── fixtures/       # Test data
```

---

## 4. Module-Level Issues

### Issue: Large Files

| File | Lines | Recommendation |
|------|-------|----------------|
| rnk_app.rs | 754 | Split into submodules |
| diagnostics.rs | 678 | Split by command |
| tool_orchestrator.rs | 550 | Split by phase |
| strategy.rs | 550 | One file per strategy |

### Issue: Deep Nesting

**Affected Modules:**
- `sage-tools/src/tools/file_ops/`
- `sage-tools/src/tools/database/sql/`

**Recommendation:**
Flatten module hierarchy where possible.

---

## 5. API Boundary Issues

### Issue: Leaky Abstractions

**Problem:** Internal types exposed in public APIs.

**Example:**
```rust
// Exposes internal LlmProvider type
pub fn create_client(provider: LlmProvider) -> Client
```

**Recommendation:**
```rust
// Use configuration instead
pub fn create_client(config: ClientConfig) -> Client
```

---

### Issue: SDK Simplicity

**Current SDK:**
- Minimal wrapper around core
- Missing common use cases

**Recommendation:**
Add high-level APIs:
```rust
// Current (verbose)
let config = ConfigBuilder::new()...build()?;
let client = SageClient::new(config)?;
let agent = client.create_agent()?;
let result = agent.execute(task).await?;

// Proposed (simple)
let sage = Sage::quick_start()?;
let result = sage.run("task").await?;
```

---

## 6. Concurrency Patterns

### Good Patterns
- `CancellationToken` for graceful shutdown
- Async channel communication
- Task-based parallelism

### Issues
1. **Lock Duration:** Some locks held across async boundaries
2. **Backpressure:** Limited backpressure in streaming
3. **Task Tracking:** Background tasks may not be properly tracked

---

## 7. Error Architecture

### Current
- `SageError` unified error type
- `ToolError` for tool-specific errors
- `anyhow` for ad-hoc errors

### Issues
1. Mixed error handling strategies
2. Error context sometimes lost
3. User-facing vs internal errors not always separated

### Recommendation
```rust
// Layered error architecture
pub enum UserError {      // User-facing, actionable
    ConfigurationError(String),
    ToolFailed { tool: String, reason: String },
}

pub enum InternalError {  // For logging/debugging
    LlmConnectionFailed(source: Box<dyn Error>),
    ToolPanic { tool: String, backtrace: Backtrace },
}
```

---

## 8. State Management

### Current State Locations
- `Session` - Conversation state
- `AgentState` - Agent execution state
- `ToolRegistry` - Available tools
- Various caches

### Issue: State Synchronization
Multiple state holders may become inconsistent.

### Recommendation
Consider centralized state store:
```rust
pub struct AppState {
    session: Arc<RwLock<Session>>,
    agent: Arc<RwLock<AgentState>>,
    tools: Arc<ToolRegistry>,
}
```

---

## 9. Recommendations

### Immediate (Architecture Hygiene)
1. Document module dependencies
2. Add architecture diagrams
3. Define API stability guarantees

### Short-term (Refactoring)
4. Split large files (>500 lines)
5. Flatten deep module hierarchies
6. Standardize error handling

### Medium-term (Restructuring)
7. Consider crate splitting for sage-core
8. Implement feature flags
9. Add SDK convenience APIs

### Long-term (Evolution)
10. Plugin API stabilization
11. Versioned configuration schema
12. Migration guides for breaking changes

---

## Architecture Checklist

- [ ] Each crate has single responsibility
- [ ] No circular dependencies
- [ ] Clear public API boundaries
- [ ] Consistent error handling
- [ ] Documented architecture decisions
- [ ] Performance-critical paths identified
- [ ] State management centralized
- [ ] Feature flags for optional modules

---

## Architecture Decision Records Needed

1. **ADR-001**: Unified Executor Design
2. **ADR-002**: MCP Integration Approach
3. **ADR-003**: Session Persistence Strategy
4. **ADR-004**: Tool Permission Model
5. **ADR-005**: Checkpoint/Recovery System
