# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sage Agent is a Rust-based LLM agent system for software engineering tasks, originally inspired by ByteDance's Trae Agent. This is a multi-crate Rust workspace with async architecture built on Tokio.

## Build and Development Commands

### Building
```bash
# Build in debug mode
cargo build

# Build optimized release
make release

# Install CLI globally
make install
# or
cargo install --path crates/sage-cli
```

### Testing
```bash
# Run all tests
make test
# or
cargo test

# Unit tests only
make test-unit

# Integration tests only  
make test-int
```

### Development
```bash
# Run the CLI in dev mode (interactive)
make dev
# or
cargo run -p sage-cli

# Run with a task
cargo run -p sage-cli -- "your task"

# Print mode (non-interactive, execute and exit)
cargo run -p sage-cli -- -p "your task"

# Resume most recent session
cargo run -p sage-cli -- -c

# Code quality checks
make clippy        # Linting
make fmt          # Formatting
make check        # Type checking without building

# Full development cycle
make quick        # fmt + clippy + test
make ci          # Full CI check
```

### Examples
```bash
# Run example code
make examples
cargo run --example basic_usage
cargo run --example markdown_demo
cargo run --example ui_demo
cargo run --example trajectory_demo
```

## Architecture

This is a Rust workspace with four main crates:

- **`sage-core/`**: Core library containing agent execution engine, LLM providers, tool system, and UI components
- **`sage-cli/`**: Command-line interface with interactive mode and progress indicators  
- **`sage-sdk/`**: High-level SDK for programmatic integration
- **`sage-tools/`**: Collection of built-in tools (bash, edit, json_edit, codebase_retrieval, task management, etc.)

### Key Directories
- `crates/sage-core/src/agent/`: Core agent execution logic and state management
- `crates/sage-core/src/llm/`: LLM client implementations for multiple providers
- `crates/sage-core/src/tools/`: Tool registry and execution system
- `crates/sage-core/src/ui/`: Terminal UI components with animations
- `examples/`: Usage examples for SDK and CLI features
- `docs/`: Comprehensive documentation structure
- `trajectories/`: Execution recording files (gitignored)

## Configuration

The project uses JSON configuration files:
- Main config: `sage_config.json` 
- Example: `sage_config.json.example`
- Template: `configs/sage_config.example.json`

Configuration supports multiple LLM providers (OpenAI, Anthropic, Google) with environment variable substitution.

## Key Features

- **Multi-LLM Support**: OpenAI, Anthropic, Google, and other providers
- **Rich Tool Ecosystem**: Built-in tools for code editing, bash execution, codebase retrieval
- **Interactive CLI**: Terminal UI with animations and progress indicators  
- **Trajectory Recording**: Complete execution tracking and replay
- **SDK Integration**: Programmatic usage through high-level SDK
- **Async Architecture**: Built on Tokio for concurrent operations

## Documentation

The project has extensive documentation in `docs/`:
- `docs/user-guide/`: User documentation
- `docs/development/`: Developer guides and contribution info
- `docs/architecture/`: System design documentation
- `docs/api/`: API reference
- `docs/planning/`: Project roadmap and TODO lists

## Testing Strategy

Tests are organized as:
- Unit tests: `cargo test --lib`
- Integration tests: `cargo test --test integration_test`
- Examples serve as integration tests: `make examples`

## Development Guidelines

- Follow Rust 2024 edition standards
- Use workspace dependencies defined in root `Cargo.toml`
- All crates share common version (0.1.0) and metadata
- Async code uses Tokio runtime
- Error handling with `anyhow` and `thiserror`
- Logging with `tracing`
- CLI built with `clap`

## Code Style Rules

### No Backward Compatibility
- **NEVER add deprecated type aliases or shims for backward compatibility**
- When renaming types, functions, or APIs, update all references directly
- Do not preserve old names with `#[deprecated]` annotations
- Breaking changes are acceptable - update the version number accordingly

### Rust Naming Conventions (RFC 430)
- Treat acronyms as words in PascalCase: `LlmClient` (not `LLMClient`), `HttpRequest` (not `HTTPRequest`)
- Correct examples: `LlmProvider`, `SseDecoder`, `McpServer`, `CliConsole`
- Keep files under 200 lines - split large modules into focused submodules

## Rust Coding Rules (Mandatory)

These rules are derived from real bugs found in this codebase. **Every rule corresponds to a past incident.**

### Type Conversions
- **NEVER use `as` for narrowing integer casts** (e.g., `u32 as i32`, `u64 as u32`). Always use `TryFrom`/`try_from` and handle the error. Example: `i32::try_from(pid)?` instead of `pid as i32`
- **NEVER use `as` to convert `f64` to integer types.** Always validate with `is_finite()` and range checks first, then cast. Provide a sensible default for invalid values (NaN, Infinity, negative)

### Collections: No Vec::remove(0)
- **NEVER use `Vec::remove(0)` or `Vec::drain(..1)` for FIFO/ring-buffer patterns.** It is O(n) per call and O(n²) in loops
- Use `VecDeque` with `push_back()`/`pop_front()` instead
- Similarly, **NEVER use `String::remove(0)` in a loop.** Use `drain(..n)` to remove multiple leading chars in one O(n) pass

### Async Task Spawning
- **NEVER call `tokio::spawn()` without storing the returned `JoinHandle`.** Fire-and-forget tasks leak resources and silently swallow panics
- Store handles in the parent struct and `abort()` them in `Drop`
- If a task is truly fire-and-forget, document why explicitly

### Stub/Unimplemented Functions
- **NEVER return `Ok(())` from a function that doesn't actually do what its signature promises.** If a feature is not implemented, return `Err` with a clear message, or mark the function `unimplemented!()`/`todo!()`
- Specifically: do not accept `&mut Command` and silently skip modifying it. Do not use `std::mem::forget` to "keep things alive" as a workaround

### Resource Management
- **NEVER use `std::mem::forget` to extend lifetimes.** It leaks the resource. Use `Arc`, struct fields, or explicit ownership transfer instead

### Shared Types (Layer 0)
- **Cross-module shared types MUST be defined in `crate::types/`** (e.g., `MessageRole`, `ToolCall`, `LlmProvider`, `TodoItem`). Original modules re-export via `pub use crate::types::TypeName;`
- **`types/` has zero dependencies on other sage-core modules** — only external crates (serde, chrono, thiserror, etc.)
- **NEVER duplicate a type definition across modules.** If two modules need the same type, move it to `types/` and re-export
- **Generic type names MUST have a domain prefix** when shared: `ToolError` not `Error`, `TodoStatus` not `Status`
- **NEVER create identity type aliases** (e.g., `type Foo = Foo;`) — use re-exports instead