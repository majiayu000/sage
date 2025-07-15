# Sage Agent

🤖 **Sage Agent** is a powerful LLM-based agent system for general-purpose software engineering tasks, built in Rust with modern async architecture and clean design patterns.

## ✨ Features

- **Multi-LLM Support**: Compatible with OpenAI, Anthropic, and other LLM providers
- **Rich Tool Ecosystem**: Built-in tools for code editing, bash execution, codebase retrieval, and task management
- **Interactive CLI**: Beautiful terminal interface with animations and progress indicators
- **SDK Integration**: High-level SDK for programmatic usage
- **Trajectory Recording**: Complete execution tracking and replay capabilities
- **Markdown Rendering**: Terminal-based markdown display with syntax highlighting
- **Task Management**: Built-in task planning and progress tracking
- **Clean Architecture**: Modular design with clear separation of concerns

## 🏗️ Architecture

The project is organized as a Rust workspace with four main crates:

- **`sage-core`**: Core library with agent execution, LLM integration, and tool management
- **`sage-cli`**: Command-line interface with interactive mode and rich UI
- **`sage-sdk`**: High-level SDK for programmatic integration
- **`sage-tools`**: Collection of built-in tools for various tasks

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/sage-agent
cd sage-agent

# Build the project
cargo build --release

# Install the CLI
cargo install --path crates/sage-cli
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

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines for details on:

- Code style and conventions
- Testing requirements
- Pull request process
- Issue reporting

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://rust-lang.org/) and modern async patterns
- Powered by leading LLM providers
- Inspired by the need for intelligent development automation

---

**Sage Agent** - Empowering developers with intelligent automation 🚀
