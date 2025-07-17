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
# Run the CLI in dev mode
make dev
# or
cargo run --bin sage

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