# Sage Agent

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Build-Passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/Version-0.1.0-blue.svg)]()

**🌐 语言 / Language**

[![English](https://img.shields.io/badge/English-4285F4?style=for-the-badge&logo=google-translate&logoColor=white)](README.md) [![中文](https://img.shields.io/badge/中文-FF6B6B?style=for-the-badge&logo=google-translate&logoColor=white)](README_zh.md)

</div>

---

🤖 **Sage Agent** 是一个强大的基于 LLM 的软件工程 Agent 系统，设计灵感来自 Claude Code。使用 Rust 构建，采用现代异步架构，提供完整的 CLI、SDK 和可扩展的工具系统。

## ✨ 功能特性

| 🤖 **AI 集成** | 🛠️ **开发工具** | 💬 **用户体验** |
|:---:|:---:|:---:|
| 8 个 LLM 提供商 | 40+ 内置工具 | 交互式聊天模式 |
| Prompt 缓存 | Slash 命令 | 会话恢复 |
| 流式响应 | 文件编辑/读取/写入 | 执行轨迹记录 |
| 成本跟踪 | Glob/Grep 搜索 | 终端 UI |

### 核心亮点

- **多 LLM 支持**：OpenAI、Anthropic、Google、Azure、OpenRouter、Ollama、豆包、GLM
- **Claude Code 风格命令**：16 个 slash 命令（`/resume`、`/undo`、`/cost`、`/plan` 等）
- **丰富的工具生态**：Bash、文件操作、Web 搜索、任务管理等
- **交互式聊天模式**：支持上下文保持的连续对话
- **会话管理**：支持交互式选择恢复历史会话
- **执行轨迹记录**：完整的执行跟踪，用于调试和回放

## 🚀 快速开始

```bash
# 从源码安装
git clone https://github.com/majiayu000/sage
cd sage
cargo install --path crates/sage-cli

# 启动交互模式
sage interactive

# 运行单次任务
sage run "创建一个 Python 斐波那契脚本"

# 使用 unified 模式（Claude Code 风格）
sage unified "审查这个代码库"
```

### 配置

创建 `sage_config.json`：

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

## 📜 Slash 命令

在 `run` 和 `interactive` 模式中都可以使用 slash 命令：

| 命令 | 描述 | 类型 |
|------|------|------|
| `/resume` | 恢复历史会话（交互式选择） | 交互式 |
| `/resume <id>` | 通过 ID 恢复指定会话 | 交互式 |
| `/resume --all` | 显示所有项目的会话 | 交互式 |
| `/commands` | 列出所有可用的 slash 命令 | 本地 |
| `/cost` | 显示会话成本和 token 使用量 | 本地 |
| `/context` | 显示上下文窗口使用情况 | 本地 |
| `/status` | 显示 agent 状态和版本 | 本地 |
| `/help` | 显示 AI 帮助信息 | 提示 |
| `/undo` | 撤销最后的文件更改（git restore） | 提示 |
| `/clear` | 清除对话历史 | 特殊 |
| `/compact` | 总结并压缩上下文 | 提示 |
| `/checkpoint [name]` | 创建状态检查点 | 提示 |
| `/restore [id]` | 恢复到检查点 | 提示 |
| `/init` | 初始化 .sage 目录 | 提示 |
| `/config` | 显示/修改配置 | 提示 |
| `/plan [open\|clear\|create]` | 查看/管理执行计划 | 提示 |
| `/tasks` | 列出后台任务 | 提示 |

### 自定义命令

在 `.sage/commands/` 或 `~/.config/sage/commands/` 中创建自定义 slash 命令：

```markdown
---
name: review
description: 审查代码更改
---

请审查以下代码更改：
$ARGUMENTS

重点关注：
1. 代码质量
2. 潜在 bug
3. 性能问题
```

## 🛠️ 可用工具

### 文件操作
| 工具 | 描述 |
|------|------|
| `Read` | 读取文件，支持行号和分页 |
| `Write` | 创建/覆盖文件 |
| `Edit` | Claude Code 风格的字符串替换编辑 |
| `Glob` | 快速文件模式匹配（`**/*.rs`、`src/**/*.ts`） |
| `Grep` | 正则搜索，支持上下文（`-A`、`-B`、`-C` 参数） |
| `NotebookEdit` | 编辑 Jupyter notebooks |

### 进程/Shell
| 工具 | 描述 |
|------|------|
| `Bash` | 执行 shell 命令，支持后台运行 |
| `KillShell` | 终止后台 shell 进程 |
| `Task` | 启动专门的 agent（Explore、Plan） |
| `TaskOutput` | 获取后台任务输出 |

### 任务管理
| 工具 | 描述 |
|------|------|
| `TodoWrite` | 创建/管理结构化任务列表 |
| `ViewTasklist` | 显示当前任务 |
| `AddTasks` | 添加新任务 |
| `UpdateTasks` | 更新任务状态 |
| `TaskDone` | 标记任务完成 |

### 网络工具
| 工具 | 描述 |
|------|------|
| `WebSearch` | 搜索网络 |
| `WebFetch` | 获取网页内容为 markdown |
| `Browser` | 在默认浏览器打开 URL |

### 规划与交互
| 工具 | 描述 |
|------|------|
| `EnterPlanMode` | 进入只读规划模式 |
| `ExitPlanMode` | 退出并批准计划 |
| `AskUserQuestion` | 向用户请求输入 |

## 💬 交互模式

```bash
sage interactive
```

**内置命令：**
- `help` - 显示帮助和 slash 命令
- `config` - 显示配置
- `status` - 显示系统状态
- `new` - 开始新对话
- `clear` - 清屏
- `exit` - 退出交互模式

**会话示例：**
```
> 创建一个 hello world Python 脚本
[Agent 创建脚本]

> 现在给它添加错误处理
[Agent 基于上下文修改脚本]

> /cost
Session Cost & Usage
====================
[显示 token 使用量和估算成本]

> /resume
[交互式会话选择器]
```

## 📦 SDK 使用

```rust
use sage_sdk::{SageAgentSDK, RunOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = SageAgentSDK::with_config_file("sage_config.json")?
        .with_working_directory("./my-project")
        .with_max_steps(10);

    // 简单执行
    let result = sdk.run("创建一个 README 文件").await?;

    // 带选项执行
    let options = RunOptions::new()
        .with_trajectory(true)
        .with_trajectory_path("./debug.jsonl");
    let result = sdk.run_with_options("分析代码库", options).await?;

    if result.is_success() {
        println!("✅ 完成，共 {} 步", result.statistics().total_steps);
    }

    Ok(())
}
```

## 🔄 会话管理

### JSONL 会话存储
会话存储在 `~/.sage/sessions/`，使用 JSONL 格式：
- 消息历史和工具调用
- Token 使用统计
- 文件变更跟踪
- Git 分支上下文

### 恢复会话
```bash
# 交互式选择
sage run "/resume"

# 恢复指定会话
sage run "/resume abc123-session-id"

# 显示所有项目
sage run "/resume --all"
```

### 执行轨迹记录
```bash
# 自动生成轨迹
sage run "调试认证模块"
# 保存到: trajectories/trajectory_YYYYMMDD_HHMMSS.jsonl

# 自定义路径
sage run "任务" --trajectory-file debug.jsonl
```

## 🔧 CLI 命令

```bash
sage run <task>              # 单次任务执行
sage interactive             # 交互式聊天模式
sage unified [task]          # Claude Code 风格统一执行
sage config show|validate|init  # 配置管理
sage trajectory list|show|stats # 轨迹管理
sage tools                   # 列出可用工具
```

### 常用选项
```bash
--provider <name>      # LLM 提供商 (anthropic, openai, google 等)
--model <name>         # 使用的模型
--api-key <key>        # API 密钥
--max-steps <n>        # 最大执行步数
--working-dir <path>   # 工作目录
--config-file <path>   # 配置文件
--trajectory-file <path> # 轨迹输出文件
--verbose              # 详细输出
```

## 🌐 LLM 提供商

| 提供商 | 默认模型 | 特性 |
|--------|----------|------|
| Anthropic | claude-sonnet-4-20250514 | Prompt 缓存，10 次最大重试 |
| OpenAI | gpt-4 | 并行工具调用 |
| Google | gemini-1.5-pro | - |
| Azure OpenAI | gpt-4 | API 版本 2024-02-15 |
| OpenRouter | anthropic/claude-3.5-sonnet | 多模型路由 |
| Ollama | llama2 | 本地模型 |
| 豆包 | doubao-pro-4k | 字节跳动 |
| GLM/智谱 | - | 自定义提供商 |

## 🏗️ 架构

```
sage/
├── crates/
│   ├── sage-core/      # 核心库
│   │   ├── agent/      # Agent 执行
│   │   ├── commands/   # Slash 命令系统
│   │   ├── llm/        # LLM 提供商
│   │   ├── session/    # 会话管理
│   │   ├── tools/      # 工具注册
│   │   └── ui/         # 终端 UI
│   ├── sage-cli/       # 命令行界面
│   ├── sage-sdk/       # 高级 SDK
│   └── sage-tools/     # 内置工具
├── examples/           # 使用示例
├── docs/               # 文档
└── configs/            # 配置模板
```

## 🔄 项目起源

本项目灵感来自：
- **[Trae Agent](https://github.com/bytedance/trae-agent)** - 字节跳动的 Python LLM Agent
- **[Claude Code](https://claude.ai/code)** - Anthropic 的 CLI 工具设计模式
- **[Augment Code](https://www.augmentcode.com/)** - AI 代码助手模式

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE)。

---

**Sage Agent** - Rust 构建的 AI 软件工程助手。🦀✨
