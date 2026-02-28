# ADR-0001: Multi-Crate Workspace Architecture

## Status

Accepted

## Context

Sage Agent is a complex system with multiple components serving different purposes:
- A core library providing agent execution, LLM integration, and tool management
- A command-line interface for user interaction
- An SDK for programmatic integration
- A collection of reusable tools

We needed to decide how to structure the codebase to:
1. Enable code reuse across different use cases (CLI, SDK, embedded usage)
2. Maintain clear separation of concerns
3. Allow independent development and testing of components
4. Support flexible deployment scenarios (CLI only, library integration, etc.)
5. Follow Rust ecosystem best practices

## Decision

We adopted a **multi-crate workspace** structure with four distinct crates:

### Crate Organization

1. **`sage-core`**: Core library
   - Agent execution engine
   - LLM client and provider abstractions
   - Tool system (registry, executor, permissions)
   - UI components (markdown rendering, animations)
   - Configuration management
   - No dependencies on CLI or SDK

2. **`sage-cli`**: Command-line interface
   - Interactive and one-shot modes
   - Session management and resume functionality
   - Slash commands (`/resume`, `/cost`, `/plan`, etc.)
   - Progress indicators and terminal UI
   - Depends on: `sage-core`, `sage-tools`, `sage-sdk`

3. **`sage-sdk`**: High-level SDK
   - Simplified API for programmatic usage
   - Agent builder and configuration helpers
   - Convenience wrappers for common operations
   - Depends on: `sage-core`, `sage-tools`

4. **`sage-tools`**: Built-in tools collection
   - 40+ reusable tools (bash, file operations, web search, etc.)
   - Database tools (MongoDB, SQL)
   - Network tools (HTTP client, web fetch, browser automation)
   - Planning and task management tools
   - Depends on: `sage-core`

### Workspace Configuration

All crates share common metadata and dependencies defined in the root `Cargo.toml`:
- Unified version: `0.1.0`
- Rust edition: `2024`
- Shared workspace dependencies (tokio, serde, anyhow, etc.)
- Workspace-level resolver (`resolver = "2"`)

## Consequences

### Positive

1. **Clear Dependency Graph**: Core has no dependencies on CLI/SDK, enabling flexible integration
2. **Code Reuse**: Tools and core functionality can be used in multiple contexts
3. **Independent Testing**: Each crate can be tested independently
4. **Flexible Deployment**:
   - Use `sage-cli` for standalone CLI
   - Use `sage-sdk` for embedding in applications
   - Use `sage-core` directly for maximum control
5. **Build Optimization**: Cargo can build and cache crates independently
6. **Clear Boundaries**: Each crate has well-defined responsibilities
7. **Workspace Benefits**:
   - Shared dependencies reduce duplication
   - Unified version management
   - Single `Cargo.lock` for consistent dependency resolution

### Negative

1. **Increased Complexity**: More `Cargo.toml` files to maintain
2. **Circular Dependency Risk**: Need to carefully manage dependencies between crates
3. **Learning Curve**: New contributors must understand the crate structure
4. **Build Time**: Initial full workspace builds can be slower than monolithic structure

### Maintenance Considerations

1. **Version Synchronization**: All crates use workspace version, simplifying releases
2. **Dependency Management**: Workspace dependencies prevent version drift
3. **Example Organization**: Examples are shared at workspace level, testing integration
4. **Documentation**: Each crate needs its own documentation while maintaining consistency

### Alternative Approaches Considered

1. **Monolithic Single Crate**: Rejected due to lack of modularity and reusability
2. **Binary-only with lib.rs**: Rejected as it doesn't separate CLI from SDK concerns
3. **More Fine-grained Crates**: Rejected as it would add unnecessary complexity without clear benefits

## References

- Root `Cargo.toml`: Workspace configuration
- `crates/sage-core/Cargo.toml`: Core library dependencies
- `crates/sage-cli/Cargo.toml`: CLI binary configuration
- `crates/sage-sdk/Cargo.toml`: SDK library configuration
- `crates/sage-tools/Cargo.toml`: Tools collection
