# Sage Agent

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Build-Passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/Version-0.1.0-blue.svg)]()

</div>



**🌐 Language / 语言**

[![English](https://img.shields.io/badge/English-4285F4?style=for-the-badge&logo=google-translate&logoColor=white)](README.md) [![中文](https://img.shields.io/badge/中文-FF6B6B?style=for-the-badge&logo=google-translate&logoColor=white)](README_zh.md)



---

🤖 **Sage Agent** is a powerful LLM-based agent system for general-purpose software engineering tasks, built in Rust with modern async architecture and clean design patterns.



## 🔄 Project Origin

This project is a **Rust rewrite** of the original [**Trae Agent**](https://github.com/bytedance/trae-agent) by ByteDance. While maintaining the core functionality and philosophy of the original Python-based agent, Sage Agent brings:

- **🚀 Performance**: Rust's zero-cost abstractions and memory safety
- **⚡ Concurrency**: Modern async/await patterns with Tokio
- **🛡️ Type Safety**: Compile-time guarantees and robust error handling
- **🏗️ Modularity**: Clean architecture with well-defined service boundaries

We extend our gratitude to the ByteDance team and the open-source community for creating the foundational Trae Agent project that inspired this implementation.

## 📋 Table of Contents

- [✨ Features](#-features)
- [🏗️ Architecture](#️-architecture)
- [🚀 Quick Start](#-quick-start)
  - [System Requirements](#system-requirements)
  - [Installation](#installation)
  - [Configuration](#configuration)
  - [Basic Usage](#basic-usage)
- [🛠️ Available Tools](#️-available-tools)
- [📖 Examples](#-examples)
- [📊 Trajectory Recording](#-trajectory-recording)
- [🎨 Advanced Features](#-advanced-features)
- [⚡ Performance Optimization](#-performance-optimization)
- [🔧 Development](#-development)
- [📚 Documentation](#-documentation)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

## ✨ Features

<div align="center">

| 🤖 **AI Integration** | 🛠️ **Developer Tools** | 🎨 **User Experience** |
|:---:|:---:|:---:|
| Multi-LLM Support<br/>*(OpenAI, Anthropic, Google)* | Rich Tool Ecosystem<br/>*(Code editing, Bash, Retrieval)* | Interactive CLI<br/>*(Animations, Progress indicators)* |
| Smart Context Handling | Task Management System | Terminal Markdown Rendering |
| Trajectory Recording | SDK Integration | Beautiful UI Components |

</div>

### 🔥 Key Highlights

- **🌐 Multi-LLM Support**: Compatible with OpenAI, Anthropic, Google, and other LLM providers
- **🛠️ Rich Tool Ecosystem**: Built-in tools for code editing, bash execution, codebase retrieval, and task management
- **💻 Interactive CLI**: Beautiful terminal interface with animations and progress indicators
- **📦 SDK Integration**: High-level SDK for programmatic usage
- **📊 Trajectory Recording**: Complete execution tracking and replay capabilities
- **📝 Markdown Rendering**: Terminal-based markdown display with syntax highlighting
- **📋 Task Management**: Built-in task planning and progress tracking
- **🏗️ Clean Architecture**: Modular design with clear separation of concerns

## 🏗️ Architecture

The project is organized as a Rust workspace with four main crates:

- **`sage-core`**: Core library with agent execution, LLM integration, and tool management
- **`sage-cli`**: Command-line interface with interactive mode and rich UI
- **`sage-sdk`**: High-level SDK for programmatic integration
- **`sage-tools`**: Collection of built-in tools for various tasks

## 🚀 Quick Start

> **💡 TL;DR**: `cargo install sage-cli && sage` - Get started in seconds!



```bash
# 🚀 One-line installation
cargo install --git https://github.com/majiayu000/sage sage-cli

# 🎯 Start interactive mode
sage

# ✨ Or run a specific task
sage run "Create a Python script that calculates fibonacci numbers"
```



### System Requirements

- **Rust**: 1.85+ (latest stable recommended)
- **Operating System**: Linux, macOS, Windows
- **Memory**: Minimum 4GB RAM (8GB+ recommended)
- **API Keys**: API keys for your chosen LLM providers

### Installation

#### Method 1: Build from Source

```bash
# Clone the repository
git clone https://github.com/majiayu000/sage
cd sage-agent

# Build the project
cargo build --release

# Install the CLI
cargo install --path crates/sage-cli
```

#### Method 2: Install via Cargo

```bash
# Install from crates.io (if published)
cargo install sage-cli

# Or install from Git repository
cargo install --git https://github.com/majiayu000/sage sage-cli
```

#### Verify Installation

```bash
# Check version
sage --version

# Show help
sage --help
```

### Configuration

Create a configuration file `sage_config.json`:

```json
{
  "providers": {
    "openai": {
      "api_key": "${OPENAI_API_KEY}",
      "base_url": "https://api.openai.com/v1"
    }
  },
  "default_provider": "openai",
  "model_parameters": {
    "model": "gpt-4",
    "temperature": 0.7,
    "max_tokens": 4000
  },
  "max_steps": 20,
  "working_directory": "."
}
```

### Basic Usage

#### CLI Mode

```bash
# Interactive mode (default)
sage

# Run a specific task
sage run "Create a Python script that calculates fibonacci numbers"

# With custom configuration
sage --config-file my_config.json run "Analyze this codebase structure"
```

#### SDK Usage

```rust
use sage_sdk::{SageAgentSDK, RunOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SDK instance
    let sdk = SageAgentSDK::new()?
        .with_provider_and_model("openai", "gpt-4", None)?
        .with_working_directory("./my-project")
        .with_max_steps(10);

    // Execute a task
    let result = sdk.run("Create a README file for this project").await?;
    
    if result.is_success() {
        println!("✅ Task completed successfully!");
        println!("📊 Used {} tokens in {} steps", 
                 result.statistics().total_tokens,
                 result.statistics().total_steps);
    }
    
    Ok(())
}
```

## 🛠️ Available Tools

Sage Agent comes with a comprehensive set of built-in tools:

- **`bash`**: Execute shell commands and scripts
- **`edit`**: Create and modify files with precise editing capabilities
- **`json_edit`**: Specialized JSON file editing
- **`codebase_retrieval`**: Intelligent code search and context retrieval
- **`sequential_thinking`**: Step-by-step reasoning and planning
- **`task_done`**: Mark tasks as completed
- **Task Management**: `view_tasklist`, `add_tasks`, `update_tasks`, `reorganize_tasklist`

## 📖 Examples

The `examples/` directory contains various usage examples:

- **`basic_usage.rs`**: Simple SDK usage patterns
- **`custom_tool.rs`**: Creating custom tools
- **`markdown_demo.rs`**: Terminal markdown rendering
- **`ui_demo.rs`**: Interactive UI components

Run examples with:

```bash
cargo run --example basic_usage
cargo run --example markdown_demo
cargo run --example trajectory_demo
```

## 📊 Trajectory Recording

Sage Agent automatically records detailed execution trajectories for debugging and analysis:

```bash
# Automatically generate trajectory files
sage run "Debug authentication module"
# Saved to: trajectories/trajectory_20250612_220546.json

# Custom trajectory file
sage run "Optimize database queries" --trajectory-file optimization_debug.json
```

Trajectory files contain:

- **LLM Interactions**: All messages, responses, and tool calls
- **Agent Steps**: State transitions and decision points
- **Tool Usage**: Which tools were called and their results
- **Metadata**: Timestamps, token usage, and execution metrics

## 🎨 Advanced Features

### Interactive Mode

In interactive mode, you can:

- Enter any task description to execute
- Use `status` to view agent information
- Use `help` to get available commands
- Use `clear` to clear the screen
- Use `exit` or `quit` to end the session

### Multi-Provider Support

```bash
# Use OpenAI
sage run "Create Python script" --provider openai --model gpt-4

# Use Anthropic
sage run "Code review" --provider anthropic --model claude-3-5-sonnet

# Use custom working directory
sage run "Add unit tests" --working-dir /path/to/project
```

### Configuration Priority

1. Command line arguments (highest priority)
2. Configuration file values
3. Environment variables
4. Default values (lowest priority)

## ⚡ Performance Optimization

### Best Practices

- **Concurrent Processing**: Sage Agent uses Tokio async runtime for efficient concurrent operations
- **Memory Management**: Rust's zero-cost abstractions ensure minimal runtime overhead
- **Caching Strategy**: Intelligent caching of LLM responses and tool results for improved performance
- **Streaming Processing**: Support for streaming LLM responses for better user experience

### Configuration Tuning

```json
{
  "model_parameters": {
    "temperature": 0.1,        // Lower randomness for more consistent results
    "max_tokens": 2000,        // Adjust based on task complexity
    "stream": true             // Enable streaming responses
  },
  "max_steps": 15,             // Limit max steps to control costs
  "timeout_seconds": 300       // Set reasonable timeout
}
```

### Monitoring and Logging

```bash
# Enable verbose logging
RUST_LOG=sage_core=debug,sage_cli=info cargo run

# Monitor token usage
sage run "Task description" --show-stats

# Performance profiling
RUST_LOG=trace cargo run --release
```

## 🔧 Development

### Building

```bash
# Build all crates
cargo build

# Build with optimizations
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Project Structure

```
sage-agent/
├── crates/
│   ├── sage-core/          # Core library
│   │   ├── src/agent/      # Agent execution logic
│   │   ├── src/llm/        # LLM client implementations
│   │   ├── src/tools/      # Tool system
│   │   └── src/ui/         # Terminal UI components
│   ├── sage-cli/           # Command-line interface
│   ├── sage-sdk/           # High-level SDK
│   └── sage-tools/         # Built-in tools collection
├── docs/                   # Comprehensive documentation
│   ├── user-guide/         # User documentation
│   ├── development/        # Developer documentation
│   ├── architecture/       # System architecture docs
│   ├── api/                # API reference
│   └── planning/           # Project planning and roadmap
├── examples/               # Usage examples
├── trajectories/           # Execution trajectory files (gitignored)
├── configs/                # Configuration templates and examples
└── Cargo.toml             # Workspace configuration
```

## 🎯 Use Cases

- **Code Generation**: Create files, functions, and entire modules
- **Code Analysis**: Understand and document existing codebases
- **Refactoring**: Modernize and improve code structure
- **Testing**: Generate and run test suites
- **Documentation**: Create comprehensive project documentation
- **Automation**: Automate repetitive development tasks

## 📝 Configuration

Sage Agent supports flexible configuration through JSON files and environment variables:

```json
{
  "providers": {
    "openai": {
      "api_key": "${OPENAI_API_KEY}",
      "base_url": "https://api.openai.com/v1"
    },
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}",
      "base_url": "https://api.anthropic.com"
    }
  },
  "default_provider": "openai",
  "model_parameters": {
    "model": "gpt-4",
    "temperature": 0.7,
    "max_tokens": 4000
  },
  "max_steps": 20,
  "working_directory": ".",
  "ui": {
    "enable_animations": true,
    "markdown_rendering": true
  },
  "trajectory": {
    "enabled": false,
    "directory": "trajectories",
    "auto_save": true,
    "save_interval_steps": 5
  }
}
```

## 📚 Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory:

- **[User Guide](docs/user-guide/)** - Installation, configuration, and usage
- **[Development Guide](docs/development/)** - Contributing and development setup
- **[Architecture Documentation](docs/architecture/)** - System design and architecture
- **[API Reference](docs/api/)** - Detailed API documentation
- **[Planning & Roadmap](docs/planning/)** - Project roadmap and TODO lists

### Quick Links
- [Getting Started](docs/user-guide/getting-started.md) - New user guide
- [Contributing Guide](docs/development/contributing.md) - How to contribute
- [TODO Lists](docs/planning/) - Current development priorities
- [MCP Integration Plan](docs/development/MCP_INTEGRATION_PLAN.md) - Model Context Protocol support
- [Documentation Consistency](docs/DOC_CONSISTENCY_GUIDE.md) - Maintaining doc consistency

## 🔧 Troubleshooting

### Common Issues

**Import Errors:**
```bash
# Try setting RUST_LOG
RUST_LOG=debug cargo run
```

**API Key Issues:**
```bash
# Verify API keys are set
echo $OPENAI_API_KEY
echo $ANTHROPIC_API_KEY

# Check configuration
sage --show-config
```

**Permission Errors:**
```bash
# Ensure proper permissions for file operations
chmod +x /path/to/your/project
```

### Environment Variables

- `OPENAI_API_KEY` - OpenAI API key
- `ANTHROPIC_API_KEY` - Anthropic API key
- `GOOGLE_API_KEY` - Google Gemini API key
- `OPENROUTER_API_KEY` - OpenRouter API key

### Development Guidelines

- Follow Rust official code style guidelines
- Add tests for new features
- Update documentation as needed
- Use appropriate type hints
- Ensure all tests pass before committing

## 🤝 Contributing

We welcome contributions! Please see our [contributing guidelines](docs/development/contributing.md) for details on:

- [Development setup](docs/development/setup.md)
- [Code style and conventions](docs/development/code-style.md)
- [Testing requirements](docs/development/testing.md)
- [Pull request process](docs/development/contributing.md#pull-requests)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

**Note**: This Rust implementation maintains compatibility with the MIT License of the original [Trae Agent](https://github.com/bytedance/trae-agent) project.

## 🙏 Acknowledgments

- **Original Inspiration**: This project is based on [Trae Agent](https://github.com/bytedance/trae-agent) by ByteDance - a pioneering LLM-based agent for software engineering tasks
- **Partial Inspiration**: [Augment Code](https://www.augmentcode.com/) - Advanced AI code assistant and context engine, providing valuable reference for agent tool system design
- Built with [Rust](https://rust-lang.org/) and modern async patterns
- Powered by leading LLM providers (Google、Anthropic、OpenAI, etc.)
- Inspired by the open-source community's commitment to intelligent development automation
- Special thanks to the Trae Agent contributors and maintainers for their foundational work
- Appreciation to the Augment Code team for their innovative work in AI-assisted development

---

**Sage Agent** - In learning.
