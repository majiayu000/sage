# Contributing to Sage Agent

Thank you for your interest in contributing to Sage Agent! This document provides guidelines and instructions for contributing to this project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Building from Source](#building-from-source)
- [Running Tests](#running-tests)
- [Code Style Guidelines](#code-style-guidelines)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting Guidelines](#issue-reporting-guidelines)
- [Commit Message Format](#commit-message-format)
- [Architecture Overview](#architecture-overview)
- [Documentation](#documentation)
- [Getting Help](#getting-help)

## Code of Conduct

This project follows the Rust community's [Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## How Can I Contribute?

There are many ways to contribute to Sage Agent:

### Reporting Bugs

Before creating bug reports, please check the [existing issues](https://github.com/majiayu000/sage/issues) to avoid duplicates. When creating a bug report, include as many details as possible:

- A clear and descriptive title
- Steps to reproduce the issue
- Expected behavior
- Actual behavior
- System information (OS, Rust version, etc.)
- Relevant logs or error messages
- Configuration files (with sensitive data removed)

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, include:

- A clear and descriptive title
- Detailed description of the proposed feature
- Explain why this enhancement would be useful
- Examples of how the feature would be used
- Possible implementation approaches (if applicable)

### Contributing Code

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following our code style guidelines
3. **Add tests** for your changes
4. **Update documentation** as needed
5. **Ensure all tests pass** and code passes linting
6. **Submit a pull request**

### Improving Documentation

Documentation improvements are always welcome! This includes:

- Fixing typos or grammatical errors
- Improving clarity of existing documentation
- Adding examples or tutorials
- Translating documentation
- Writing blog posts or guides about Sage Agent

## Development Setup

### Prerequisites

- **Rust 1.85+** (Rust 2024 edition)
- **Git**
- **Make** (optional, but recommended)

### Clone the Repository

```bash
git clone https://github.com/majiayu000/sage.git
cd sage
```

### Install Development Tools

```bash
# Update Rust toolchain
rustup update

# Install required components
rustup component add rustfmt clippy

# Or use the make target
make setup
```

### Configuration

1. Copy the example configuration:

```bash
cp sage_config.json.example sage_config.json
# or
cp configs/sage_config.example.json sage_config.json
```

2. Edit `sage_config.json` and add your API keys:

```json
{
  "default_provider": "anthropic",
  "model_providers": {
    "anthropic": {
      "model": "claude-sonnet-4-20250514",
      "api_key": "${ANTHROPIC_API_KEY}",
      "enable_prompt_caching": true
    }
  },
  "max_steps": 20,
  "working_directory": "."
}
```

3. Set environment variables:

```bash
export ANTHROPIC_API_KEY="your-api-key"
export OPENAI_API_KEY="your-api-key"  # if using OpenAI
```

## Building from Source

### Debug Build

```bash
# Using cargo
cargo build

# Using make
make build
```

### Release Build

```bash
# Using cargo
cargo build --release

# Using make
make release
```

### Install Locally

```bash
# Install the CLI globally
cargo install --path crates/sage-cli

# Or using make
make install
```

### Run in Development Mode

```bash
# Run without installing
cargo run --bin sage

# Using make
make dev

# Run with arguments
cargo run --bin sage -- interactive
make run ARGS="interactive"
```

## Running Tests

### All Tests

```bash
cargo test
# or
make test
```

### Unit Tests Only

```bash
cargo test --lib
# or
make test-unit
```

### Integration Tests Only

```bash
cargo test --test integration_test
# or
make test-int
```

### Running Examples

Examples serve as integration tests and usage demonstrations:

```bash
# Run all examples
make examples

# Run specific example
cargo run --example basic_usage
cargo run --example markdown_demo
cargo run --example ui_demo
cargo run --example trajectory_demo
```

### Test Coverage

When adding new features:

- Write unit tests for individual functions and modules
- Write integration tests for end-to-end workflows
- Add examples demonstrating the new feature
- Ensure existing tests still pass

## Code Style Guidelines

### Rust Standards

This project follows **Rust 2024 edition** standards and best practices:

- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common issues
- Follow Rust naming conventions (snake_case, CamelCase, etc.)
- Write idiomatic Rust code
- Document public APIs with doc comments (`///`)

### Code Quality Checks

Before submitting a PR, run:

```bash
# Format code
cargo fmt
# or
make fmt

# Run clippy (with warnings as errors)
cargo clippy -- -D warnings
# or
make clippy

# Run all checks
cargo check
# or
make check
```

### Quick Development Cycle

```bash
# Format, lint, and test
make quick

# Full CI check (what runs in CI)
make ci
```

### Best Practices

- **Error Handling**: Use `anyhow` for application errors, `thiserror` for library errors
- **Async Code**: Use Tokio runtime, prefer async/await over manual futures
- **Logging**: Use `tracing` for structured logging
- **Dependencies**: Use workspace dependencies defined in root `Cargo.toml`
- **Unwrap/Expect**: Avoid `unwrap()` in production code; use proper error handling
- **Testing**: Write tests alongside your code
- **Documentation**: Document complex logic and public APIs

### Workspace Structure

This is a Cargo workspace with four main crates:

- `crates/sage-core/` - Core library (agent engine, LLM clients, tools)
- `crates/sage-cli/` - Command-line interface
- `crates/sage-sdk/` - High-level SDK for integration
- `crates/sage-tools/` - Built-in tool implementations

### File Size Limits

- Keep source files under 500 lines when possible
- Break large modules into smaller, focused files
- Use submodules for complex features

## Pull Request Process

### Before Submitting

1. **Sync with upstream**: Ensure your branch is up to date with `main`
2. **Run tests**: All tests must pass (`make test`)
3. **Run linters**: Code must pass `clippy` and `fmt` checks
4. **Update documentation**: Include doc updates for new features
5. **Add changelog entry**: Document user-facing changes

### PR Requirements

- **Clear title**: Use conventional commit format (see below)
- **Description**: Explain what changed and why
- **Issue reference**: Link to related issues (e.g., "Fixes #123")
- **Tests**: Include tests for new functionality
- **Documentation**: Update relevant docs
- **Breaking changes**: Clearly mark and document breaking changes

### PR Template

```markdown
## Summary

Brief description of changes.

## Related Issues

Fixes #123
Relates to #456

## Changes

- Added feature X
- Fixed bug Y
- Updated documentation Z

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Examples updated (if applicable)

## Documentation

- [ ] Code comments added
- [ ] API documentation updated
- [ ] User guide updated (if applicable)
- [ ] CHANGELOG updated

## Breaking Changes

List any breaking changes and migration guide (if applicable).
```

### Review Process

1. Maintainers will review your PR
2. Address review feedback with new commits
3. Once approved, maintainers will merge your PR
4. Your contribution will be included in the next release

## Issue Reporting Guidelines

### Before Creating an Issue

- Search existing issues to avoid duplicates
- Check the [documentation](docs/) for answers
- Try the latest version to see if the issue is already fixed

### Bug Report Template

```markdown
**Description**
A clear and concise description of the bug.

**Steps to Reproduce**
1. Run command '...'
2. Configure setting '...'
3. See error

**Expected Behavior**
What you expected to happen.

**Actual Behavior**
What actually happened.

**System Information**
- OS: [e.g., macOS 14.0, Ubuntu 22.04]
- Rust version: [output of `rustc --version`]
- Sage version: [output of `sage --version`]
- LLM provider: [e.g., Anthropic, OpenAI]

**Configuration**
```json
// Your sage_config.json (remove sensitive data)
```

**Logs/Errors**
```
Paste relevant logs or error messages
```

**Additional Context**
Any other relevant information.
```

### Feature Request Template

```markdown
**Problem/Use Case**
Describe the problem you're trying to solve or use case you're trying to support.

**Proposed Solution**
Describe your proposed solution.

**Alternatives Considered**
Other approaches you've considered.

**Additional Context**
Any other relevant information, mockups, or examples.
```

## Commit Message Format

This project follows **[Conventional Commits](https://www.conventionalcommits.org/)** specification.

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

Must be one of:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, semicolons, etc.)
- `refactor`: Code refactoring without changing functionality
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `build`: Build system or dependency changes
- `ci`: CI/CD configuration changes
- `chore`: Other changes that don't modify src or test files
- `security`: Security fixes or improvements

### Scope (Optional)

The scope should specify the crate or component affected:

- `core`: sage-core crate
- `cli`: sage-cli crate
- `sdk`: sage-sdk crate
- `tools`: sage-tools crate
- `llm`: LLM client code
- `agent`: Agent execution logic
- `ui`: User interface components
- `docs`: Documentation

### Examples

```bash
# Feature
feat(tools): add new grep tool with regex support

# Bug fix
fix(llm): handle rate limit errors correctly

# Documentation
docs: create CONTRIBUTING.md guide (LOW-005)

# Security fix
security(tools): add URL validation to prevent SSRF attacks (CRIT-006)

# Breaking change
feat(agent)!: change agent execution model

BREAKING CHANGE: Agent.run() now returns Result<AgentOutput>
instead of AgentOutput. Update all callers to handle errors.
```

### Guidelines

- Use imperative mood ("add" not "added" or "adds")
- Don't capitalize first letter
- No period at the end
- Keep subject line under 72 characters
- Reference issues in footer (e.g., "Fixes #123", "Closes #456")
- Mark breaking changes with `!` after type/scope or in footer

## Architecture Overview

### Workspace Structure

```
sage/
├── crates/
│   ├── sage-core/          # Core library
│   │   ├── agent/          # Agent execution engine
│   │   ├── llm/            # LLM provider clients
│   │   ├── tools/          # Tool registry and execution
│   │   ├── ui/             # Terminal UI components
│   │   ├── session/        # Session management
│   │   ├── commands/       # Slash command system
│   │   ├── trajectory/     # Execution recording
│   │   └── ...
│   ├── sage-cli/           # Command-line interface
│   ├── sage-sdk/           # High-level SDK
│   └── sage-tools/         # Built-in tool implementations
├── examples/               # Usage examples
├── docs/                   # Documentation
├── configs/                # Configuration templates
└── tests/                  # Integration tests
```

### Key Components

#### Agent Execution

- **Location**: `crates/sage-core/src/agent/`
- **Purpose**: Core agent loop, state management, tool calling
- **Key Files**: `unified.rs`, `executor.rs`

#### LLM Providers

- **Location**: `crates/sage-core/src/llm/`
- **Purpose**: API clients for different LLM providers
- **Supported**: OpenAI, Anthropic, Google, Azure, OpenRouter, Ollama, Doubao, GLM

#### Tool System

- **Location**: `crates/sage-core/src/tools/` and `crates/sage-tools/src/`
- **Purpose**: Extensible tool registry and execution
- **Tools**: Bash, file operations, search, web, task management

#### Session Management

- **Location**: `crates/sage-core/src/session/`
- **Purpose**: Session storage, resume, and history tracking
- **Format**: JSONL files in `~/.sage/sessions/`

### Async Architecture

- Built on **Tokio** runtime
- Uses `async/await` throughout
- Streaming responses with futures
- Background task management

### Dependencies

Key workspace dependencies (see root `Cargo.toml`):

- **Runtime**: tokio, futures
- **HTTP**: reqwest
- **Serialization**: serde, serde_json
- **CLI**: clap, console, indicatif
- **Error handling**: anyhow, thiserror
- **Logging**: tracing, tracing-subscriber

## Documentation

### Documentation Structure

- `docs/user-guide/` - End-user documentation
- `docs/development/` - Developer guides
- `docs/architecture/` - System design documentation
- `docs/api/` - API reference
- `docs/planning/` - Project roadmap and planning
- `docs/tools/` - Tool documentation
- `CLAUDE.md` - Guidelines for AI assistants

### Writing Documentation

- Use clear, concise language
- Include code examples
- Add screenshots for UI features
- Keep documentation in sync with code
- Use proper Markdown formatting

### API Documentation

Document public APIs using Rust doc comments:

```rust
/// Executes a command in the shell.
///
/// # Arguments
///
/// * `command` - The shell command to execute
/// * `working_dir` - Optional working directory
///
/// # Returns
///
/// Returns the command output or an error.
///
/// # Examples
///
/// ```no_run
/// use sage_tools::tools::bash::BashTool;
///
/// let result = bash_tool.execute("ls -la").await?;
/// ```
pub async fn execute(&self, command: &str) -> Result<Output> {
    // Implementation
}
```

### Generating Documentation

```bash
# Generate and open Rust docs
cargo doc --open
# or
make docs
```

## Getting Help

### Resources

- **Documentation**: [docs/](docs/)
- **Examples**: [examples/](examples/)
- **Issues**: [GitHub Issues](https://github.com/majiayu000/sage/issues)
- **Discussions**: [GitHub Discussions](https://github.com/majiayu000/sage/discussions)

### Community

- Open an issue for bugs or feature requests
- Start a discussion for questions or ideas
- Check existing issues and discussions first

### Asking Questions

When asking for help:

- Be specific about what you're trying to do
- Include relevant code snippets
- Share error messages and logs
- Describe what you've already tried
- Provide system information if relevant

---

Thank you for contributing to Sage Agent! Your contributions help make this project better for everyone.
