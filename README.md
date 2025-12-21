# Sage Agent

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Build-Passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/Version-0.1.0-blue.svg)]()

**🌐 Language / 语言**

[![English](https://img.shields.io/badge/English-4285F4?style=for-the-badge&logo=google-translate&logoColor=white)](README.md) [![中文](https://img.shields.io/badge/中文-FF6B6B?style=for-the-badge&logo=google-translate&logoColor=white)](README_zh.md)

</div>

---

🤖 **Sage Agent** is a powerful LLM-based agent system for software engineering tasks, inspired by Claude Code's design patterns. Built in Rust with modern async architecture, it provides a comprehensive CLI, SDK, and extensible tool system.

## ✨ Features

| 🤖 **AI Integration** | 🛠️ **Developer Tools** | 💬 **User Experience** |
|:---:|:---:|:---:|
| 8 LLM Providers | 40+ Built-in Tools | Interactive Chat Mode |
| Prompt Caching | Slash Commands | Session Resume |
| Streaming Responses | File Edit/Read/Write | Trajectory Recording |
| Cost Tracking | Glob/Grep Search | Terminal UI |

### Key Highlights

- **Multi-LLM Support**: OpenAI, Anthropic, Google, Azure, OpenRouter, Ollama, Doubao, GLM
- **Claude Code-style Commands**: 16 slash commands (`/resume`, `/undo`, `/cost`, `/plan`, etc.)
- **Rich Tool Ecosystem**: Bash, file operations, web search, task management, and more
- **Interactive Chat Mode**: Continuous conversation with context preservation
- **Session Management**: Resume previous sessions with interactive selection
- **Trajectory Recording**: Complete execution tracking for debugging and replay

## 🚀 Quick Start

```bash
# Install from source
git clone https://github.com/majiayu000/sage
cd sage
cargo install --path crates/sage-cli

# Start interactive mode
sage interactive

# Run a one-shot task
sage run "Create a Python fibonacci script"

# Use unified mode (Claude Code style)
sage unified "Review this codebase"
```

### Configuration

Create `sage_config.json`:

```json
{
  "default_provider": "anthropic",
  "model_providers": {
    "anthropic": {
      "model": "claude-sonnet-4-20250514",
      "api_key": "${ANTHROPIC_API_KEY}",
      "enable_prompt_caching": true
    },
    "openai": {
      "model": "gpt-4",
      "api_key": "${OPENAI_API_KEY}"
    }
  },
  "max_steps": 20,
  "working_directory": "."
}
```

## 📜 Slash Commands

Use slash commands in both `run` and `interactive` modes:

| Command | Description | Type |
|---------|-------------|------|
| `/resume` | Resume a previous session (interactive selection) | Interactive |
| `/resume <id>` | Resume a specific session by ID | Interactive |
| `/resume --all` | Show sessions from all projects | Interactive |
| `/commands` | List all available slash commands | Local |
| `/cost` | Show session cost and token usage | Local |
| `/context` | Show context window usage | Local |
| `/status` | Show agent status and version | Local |
| `/help` | Show AI help information | Prompt |
| `/undo` | Undo last file changes (git restore) | Prompt |
| `/clear` | Clear conversation history | Special |
| `/compact` | Summarize and compact context | Prompt |
| `/checkpoint [name]` | Create a state checkpoint | Prompt |
| `/restore [id]` | Restore to a checkpoint | Prompt |
| `/init` | Initialize .sage directory | Prompt |
| `/config` | Show/modify configuration | Prompt |
| `/plan [open\|clear\|create]` | View/manage execution plan | Prompt |
| `/tasks` | List background tasks | Prompt |

### Custom Commands

Create custom slash commands in `.sage/commands/` or `~/.config/sage/commands/`:

```markdown
---
name: review
description: Review code changes
---

Please review the following code changes:
$ARGUMENTS

Focus on:
1. Code quality
2. Potential bugs
3. Performance issues
```

## 🛠️ Available Tools

### File Operations
| Tool | Description |
|------|-------------|
| `Read` | Read files with line numbers and pagination |
| `Write` | Create/overwrite files |
| `Edit` | Claude Code-style string replacement editing |
| `Glob` | Fast file pattern matching (`**/*.rs`, `src/**/*.ts`) |
| `Grep` | Regex search with context (`-A`, `-B`, `-C` flags) |
| `NotebookEdit` | Edit Jupyter notebooks |

### Process/Shell
| Tool | Description |
|------|-------------|
| `Bash` | Execute shell commands with background support |
| `KillShell` | Kill background shell processes |
| `Task` | Launch specialized agents (Explore, Plan) |
| `TaskOutput` | Retrieve background task output |

### Task Management
| Tool | Description |
|------|-------------|
| `TodoWrite` | Create/manage structured task lists |
| `ViewTasklist` | Display current tasks |
| `AddTasks` | Add new tasks |
| `UpdateTasks` | Update task status |
| `TaskDone` | Mark tasks completed |

### Web & Network
| Tool | Description |
|------|-------------|
| `WebSearch` | Search the web |
| `WebFetch` | Fetch webpage content as markdown |
| `Browser` | Open URLs in default browser |

### Planning & Interaction
| Tool | Description |
|------|-------------|
| `EnterPlanMode` | Enter read-only planning mode |
| `ExitPlanMode` | Exit with plan approval |
| `AskUserQuestion` | Prompt user for input |

## 💬 Interactive Mode

```bash
sage interactive
```

**Built-in Commands:**
- `help` - Show help and slash commands
- `config` - Show configuration
- `status` - Show system status
- `new` - Start new conversation
- `clear` - Clear screen
- `exit` - Exit interactive mode

**Example Session:**
```
> Create a hello world Python script
[Agent creates script]

> Now add error handling to it
[Agent modifies script with context from previous turn]

> /cost
Session Cost & Usage
====================
[Shows token usage and estimated cost]

> /resume
[Interactive session selector appears]
```

## 📦 SDK Usage

```rust
use sage_sdk::{SageAgentSDK, RunOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = SageAgentSDK::with_config_file("sage_config.json")?
        .with_working_directory("./my-project")
        .with_max_steps(10);

    // Simple execution
    let result = sdk.run("Create a README file").await?;

    // With options
    let options = RunOptions::new()
        .with_trajectory(true)
        .with_trajectory_path("./debug.jsonl");
    let result = sdk.run_with_options("Analyze codebase", options).await?;

    if result.is_success() {
        println!("✅ Completed in {} steps", result.statistics().total_steps);
    }

    Ok(())
}
```

### Non-Interactive Execution

```rust
use sage_sdk::{SageAgentSDK, UnifiedRunOptions};

let sdk = SageAgentSDK::new()?;
let options = UnifiedRunOptions::new().non_interactive(true);
let result = sdk.execute_non_interactive("Task description", options).await?;
```

## 🔄 Session Management

### JSONL Session Storage
Sessions are stored in `~/.sage/sessions/` as JSONL files with:
- Message history with tool calls
- Token usage statistics
- File change tracking
- Git branch context

### Resume Sessions
```bash
# Interactive selection
sage run "/resume"

# Resume specific session
sage run "/resume abc123-session-id"

# Show all projects
sage run "/resume --all"
```

### Trajectory Recording
```bash
# Auto-generated trajectory
sage run "Debug auth module"
# Saved to: trajectories/trajectory_YYYYMMDD_HHMMSS.jsonl

# Custom path
sage run "Task" --trajectory-file debug.jsonl
```

## 🔧 CLI Commands

```bash
sage run <task>              # One-shot task execution
sage interactive             # Interactive chat mode
sage unified [task]          # Claude Code-style unified execution
sage config show|validate|init  # Configuration management
sage trajectory list|show|stats # Trajectory management
sage tools                   # List available tools
```

### Common Options
```bash
--provider <name>      # LLM provider (anthropic, openai, google, etc.)
--model <name>         # Model to use
--api-key <key>        # API key
--max-steps <n>        # Maximum execution steps
--working-dir <path>   # Working directory
--config-file <path>   # Configuration file
--trajectory-file <path> # Trajectory output file
--verbose              # Verbose output
```

## 🌐 LLM Providers

| Provider | Default Model | Features |
|----------|---------------|----------|
| Anthropic | claude-sonnet-4-20250514 | Prompt caching, 10 max retries |
| OpenAI | gpt-4 | Parallel tool calls |
| Google | gemini-1.5-pro | - |
| Azure OpenAI | gpt-4 | API version 2024-02-15 |
| OpenRouter | anthropic/claude-3.5-sonnet | Multi-model routing |
| Ollama | llama2 | Local models |
| Doubao | doubao-pro-4k | ByteDance |
| GLM/Zhipu | - | Custom provider |

## 🏗️ Architecture

```
sage/
├── crates/
│   ├── sage-core/      # Core library
│   │   ├── agent/      # Agent execution
│   │   ├── commands/   # Slash command system
│   │   ├── llm/        # LLM providers
│   │   ├── session/    # Session management
│   │   ├── tools/      # Tool registry
│   │   └── ui/         # Terminal UI
│   ├── sage-cli/       # Command-line interface
│   ├── sage-sdk/       # High-level SDK
│   └── sage-tools/     # Built-in tools
├── examples/           # Usage examples
├── docs/               # Documentation
└── configs/            # Configuration templates
```

## 🔄 Project Origin

This project is inspired by:
- **[Trae Agent](https://github.com/bytedance/trae-agent)** - ByteDance's Python-based LLM agent
- **[Claude Code](https://claude.ai/code)** - Anthropic's CLI tool design patterns
- **[Augment Code](https://www.augmentcode.com/)** - AI code assistant patterns

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

**Sage Agent** - AI-powered software engineering in Rust. 🦀✨
