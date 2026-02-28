# Development Documentation

This section contains documentation for developers working on Sage Agent.

## 🔧 Contents

### Getting Started
- **[Development Setup](setup.md)** - Setting up your development environment
- **[Building the Project](setup.md#building)** - How to build Sage Agent
- **[Running Tests](setup.md#testing)** - Running the test suite
- **[Development Workflow](setup.md#workflow)** - Recommended development workflow

### Contributing
- **[Contributing Guide](contributing.md)** - How to contribute to the project
- **[Code Review Process](contributing.md#code-review)** - Code review guidelines
- **[Issue Guidelines](contributing.md#issues)** - Reporting bugs and requesting features
- **[Pull Request Process](contributing.md#pull-requests)** - Submitting changes

### Code Standards
- **[Code Style Guide](code-style.md)** - Rust coding standards and conventions
- **[Documentation Standards](code-style.md#documentation)** - Code documentation requirements
- **[Naming Conventions](code-style.md#naming)** - Variable and function naming
- **[Error Handling](code-style.md#error-handling)** - Error handling patterns

### Testing
- **[Testing Strategy](testing.md)** - Overall testing approach
- **[Unit Testing](testing.md#unit-tests)** - Writing unit tests
- **[Integration Testing](testing.md#integration-tests)** - Integration test guidelines
- **[Performance Testing](testing.md#performance-tests)** - Performance benchmarking
- **[Test Coverage](testing.md#coverage)** - Maintaining test coverage

### Architecture & Design
- **[MCP Integration Plan](MCP_INTEGRATION_PLAN.md)** - Model Context Protocol integration
- **[Tools Expansion Plan](TOOLS_EXPANSION_PLAN.md)** - Tool ecosystem expansion
- **[Design Patterns](design-patterns.md)** - Common design patterns used
- **[API Design Guidelines](api-design.md)** - API design principles

### Release Management
- **[Release Process](release-process.md)** - How releases are managed
- **[Versioning Strategy](release-process.md#versioning)** - Semantic versioning approach
- **[Changelog Management](release-process.md#changelog)** - Maintaining changelogs
- **[Deployment Process](release-process.md#deployment)** - Deployment procedures

## 🛠️ Development Tools

### Required Tools
- **Rust** (latest stable) - Primary programming language
- **Cargo** - Rust package manager and build tool
- **Git** - Version control system
- **IDE/Editor** - VS Code, IntelliJ IDEA, or similar

### Recommended Tools
- **rust-analyzer** - Rust language server
- **clippy** - Rust linter
- **rustfmt** - Code formatter
- **cargo-watch** - Automatic rebuilding
- **cargo-audit** - Security vulnerability scanner

### Development Scripts
```bash
# Build the project
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Format code
cargo fmt

# Run linter
cargo clippy

# Check for security vulnerabilities
cargo audit
```

## 📋 Project Structure

### Workspace Organization
```
sage-agent/
├── crates/
│   ├── sage-core/      # Core library
│   ├── sage-cli/       # Command-line interface
│   ├── sage-sdk/       # High-level SDK
│   └── sage-tools/     # Built-in tools
├── docs/               # Documentation
├── examples/           # Usage examples
├── configs/            # Configuration templates
└── trajectories/       # Execution trajectories
```

### Module Guidelines
- Each crate should have a clear, single responsibility
- Use `pub(crate)` for internal APIs
- Minimize dependencies between crates
- Follow Rust module conventions

## 🚀 Development Workflow

### Feature Development
1. **Create Issue** - Describe the feature or bug
2. **Create Branch** - Use descriptive branch names
3. **Implement Changes** - Follow coding standards
4. **Write Tests** - Ensure adequate test coverage
5. **Update Documentation** - Keep docs current
6. **Submit PR** - Follow PR template
7. **Code Review** - Address review feedback
8. **Merge** - Squash and merge when approved

### Debugging
- Use `RUST_LOG=debug` for detailed logging
- Use `cargo test -- --nocapture` for test output
- Use debugger integration in your IDE
- Add temporary debug prints with `dbg!()` macro

### Performance Profiling
- Use `cargo bench` for benchmarking
- Profile with `perf` on Linux
- Use `cargo flamegraph` for flame graphs
- Monitor memory usage with `valgrind`

## 🔍 Code Quality

### Static Analysis
- **Clippy** - Rust linter for common mistakes
- **Rustfmt** - Consistent code formatting
- **Cargo Audit** - Security vulnerability scanning
- **Cargo Deny** - License and dependency checking

### Testing Requirements
- Minimum 80% test coverage for new code
- All public APIs must have tests
- Integration tests for key workflows
- Performance regression tests

### Documentation Requirements
- All public APIs must be documented
- Include usage examples in documentation
- Keep README files up to date
- Document architectural decisions

---

For system architecture details, see the [Architecture Documentation](../architecture/).
For user-facing documentation, see the [User Guide](../user-guide/).
