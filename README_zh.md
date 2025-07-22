# Sage Agent

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Build-Passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/Version-0.1.0-blue.svg)]()
[![Status](https://img.shields.io/badge/Status-Archived-red.svg)]()

</div>

> **📦 项目已归档**
> 此项目已归档，不再积极维护。虽然代码仍可用于参考和学习目的，但不会添加新功能，也不会处理问题。如果您想继续开发，欢迎 fork 此仓库。

<div align="center">

**🌐 Language / 语言**

[![English](https://img.shields.io/badge/English-4285F4?style=for-the-badge&logo=google-translate&logoColor=white)](README.md) [![中文](https://img.shields.io/badge/中文-FF6B6B?style=for-the-badge&logo=google-translate&logoColor=white)](README_zh.md)

</div>

---

🤖 **Sage Agent** 是一个基于大语言模型的强大智能体系统，专为通用软件工程任务而设计，采用 Rust 构建，具有现代异步架构和清晰的设计模式。

## 📦 归档说明

**此项目已于 2025 年 7 月归档。**

### 为什么归档？

经过广泛的开发和实验，我们决定归档这个基于 Rust 的实现，转而采用 **基于 TypeScript 的方法**。我们的分析表明：

- **🌐 生态系统优势**：TypeScript/Node.js 拥有更丰富的 AI/LLM 生态系统，所有主要提供商都有官方 SDK
- **⚡ 开发速度**：单一语言栈显著提高开发速度和团队协作效率
- **🔧 维护简单性**：统一工具链减少复杂性和部署开销
- **📊 性能现实**：在 AI 智能体场景中，网络 I/O（LLM API 调用）是瓶颈，而非 CPU 性能

### 可用内容

此仓库包含一个**完全功能的 Rust 实现**，具有：
- ✅ 完整的并发工具执行系统
- ✅ 基于 React + Ink 的现代终端 UI
- ✅ 多 LLM 提供商支持
- ✅ 全面的工具生态系统
- ✅ 高级轨迹记录功能

### 未来开发

如果您有兴趣继续此项目：
1. **Fork 此仓库** - 所有代码都采用 MIT 许可证，可直接使用
2. **查看 TypeScript 迁移见解** - 请参阅提交历史中的分析
3. **考虑混合方法** - UI 组件和架构模式很有价值

我们相信此代码库是基于 Rust 的 AI 智能体的优秀**参考实现**，展示了并发工具执行的高级模式。



## 🔄 项目起源

本项目是字节跳动原始 [**Trae Agent**](https://github.com/bytedance/trae-agent) 的 **Rust 重写版本**。在保持原始 Python 版本智能体核心功能和设计理念的同时，Sage Agent 带来了：

- **🚀 性能提升**：Rust 的零成本抽象和内存安全保障
- **⚡ 并发优化**：基于 Tokio 的现代 async/await 模式
- **🛡️ 类型安全**：编译时保证和健壮的错误处理
- **🏗️ 模块化设计**：清晰的架构和明确的服务边界

我们向字节跳动团队和开源社区表示感谢，感谢他们创建了启发本项目实现的基础 Trae Agent 项目。

## 📋 目录

- [📦 归档说明](#-归档说明)
- [✨ 特性](#-特性)
- [🏗️ 架构](#️-架构)
- [🚀 快速开始](#-快速开始)
  - [系统要求](#系统要求)
  - [安装](#安装)
  - [配置](#配置)
  - [基本使用](#基本使用)
- [🛠️ 可用工具](#️-可用工具)
- [📖 示例](#-示例)
- [📊 轨迹记录](#-轨迹记录)
- [🎨 高级功能](#-高级功能)
- [⚡ 性能优化](#-性能优化)
- [🔧 开发](#-开发)
- [📚 文档](#-文档)
- [🔄 迁移见解](#-迁移见解)
- [🤝 贡献](#-贡献)
- [📄 许可证](#-许可证)

## ✨ 特性

<div align="center">

| 🤖 **AI 集成** | 🛠️ **开发工具** | 🎨 **用户体验** |
|:---:|:---:|:---:|
| 多 LLM 支持<br/>*(OpenAI, Anthropic, Google)* | 丰富工具生态<br/>*(代码编辑, Bash, 检索)* | 交互式 CLI<br/>*(动画, 进度指示器)* |
| 智能上下文处理 | 任务管理系统 | 终端 Markdown 渲染 |
| 轨迹记录 | SDK 集成 | 美观 UI 组件 |

</div>

### 🔥 核心亮点

- **🌐 多 LLM 支持**：兼容 OpenAI、Anthropic、Google 和其他 LLM 提供商
- **🛠️ 丰富的工具生态**：内置代码编辑、bash 执行、代码库检索和任务管理工具
- **💻 交互式 CLI**：美观的终端界面，带有动画和进度指示器
- **📦 SDK 集成**：用于编程使用的高级 SDK
- **📊 轨迹记录**：完整的执行跟踪和重放功能
- **📝 Markdown 渲染**：基于终端的 Markdown 显示和语法高亮
- **📋 任务管理**：内置任务规划和进度跟踪
- **🏗️ 清晰架构**：模块化设计，关注点分离明确

## 🏗️ 架构

项目组织为一个 Rust 工作空间，包含四个主要 crate：

- **`sage-core`**：核心库，包含智能体执行、LLM 集成和工具管理
- **`sage-cli`**：命令行界面，具有交互模式和丰富的 UI
- **`sage-sdk`**：用于编程集成的高级 SDK
- **`sage-tools`**：各种任务的内置工具集合

## 🚀 快速开始

> **💡 简单说明**: `cargo install sage-cli && sage` - 几秒钟即可开始使用！



```bash
# 🚀 一行安装
cargo install --git https://github.com/majiayu000/sage sage-cli

# 🎯 启动交互模式
sage

# ✨ 或运行特定任务
sage run "创建一个计算斐波那契数列的 Python 脚本"
```



### 系统要求

- **Rust**: 1.85+ (推荐使用最新稳定版)
- **操作系统**: Linux, macOS, Windows
- **内存**: 最少 4GB RAM（推荐 8GB+）
- **API 密钥**: 选择的 LLM 提供商的 API 密钥

### 安装

#### 方式一：从源码构建

```bash
# 克隆仓库
git clone https://github.com/majiayu000/sage
cd sage-agent

# 构建项目
cargo build --release

# 安装 CLI
cargo install --path crates/sage-cli
```

#### 方式二：使用 Cargo 直接安装

```bash
# 从 crates.io 安装（如果已发布）
cargo install sage-cli

# 或从 Git 仓库安装
cargo install --git https://github.com/majiayu000/sage sage-cli
```

#### 验证安装

```bash
# 检查版本
sage --version

# 显示帮助
sage --help
```

### 配置

创建配置文件 `sage_config.json`：

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

### 基本使用

#### CLI 模式

```bash
# 交互模式（默认）
sage

# 运行特定任务
sage run "创建一个计算斐波那契数列的 Python 脚本"

# 使用自定义配置
sage --config-file my_config.json run "分析这个代码库结构"
```

#### SDK 使用

```rust
use sage_sdk::{SageAgentSDK, RunOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 SDK 实例
    let sdk = SageAgentSDK::new()?
        .with_provider_and_model("openai", "gpt-4", None)?
        .with_working_directory("./my-project")
        .with_max_steps(10);

    // 执行任务
    let result = sdk.run("为这个项目创建一个 README 文件").await?;
    
    if result.is_success() {
        println!("✅ 任务成功完成！");
        println!("📊 使用了 {} 个 token，共 {} 步", 
                 result.statistics().total_tokens,
                 result.statistics().total_steps);
    }
    
    Ok(())
}
```

## 🛠️ 可用工具

Sage Agent 提供了一套全面的内置工具：

- **`bash`**：执行 shell 命令和脚本
- **`edit`**：创建和修改文件，具有精确的编辑功能
- **`json_edit`**：专门的 JSON 文件编辑
- **`codebase_retrieval`**：智能代码搜索和上下文检索
- **`sequential_thinking`**：逐步推理和规划
- **`task_done`**：标记任务为已完成
- **任务管理**：`view_tasklist`、`add_tasks`、`update_tasks`、`reorganize_tasklist`

## 📖 示例

`examples/` 目录包含各种使用示例：

- **`basic_usage.rs`**：简单的 SDK 使用模式
- **`custom_tool.rs`**：创建自定义工具
- **`markdown_demo.rs`**：终端 Markdown 渲染
- **`ui_demo.rs`**：交互式 UI 组件

运行示例：

```bash
cargo run --example basic_usage
cargo run --example markdown_demo
cargo run --example trajectory_demo
```

## 📊 轨迹记录

Sage Agent 自动记录详细的执行轨迹，用于调试和分析：

```bash
# 自动生成轨迹文件
sage run "调试认证模块"
# 保存到：trajectories/trajectory_20250612_220546.json

# 自定义轨迹文件
sage run "优化数据库查询" --trajectory-file optimization_debug.json
```

轨迹文件包含：

- **LLM 交互**：所有消息、响应和工具调用
- **智能体步骤**：状态转换和决策点
- **工具使用**：调用了哪些工具及其结果
- **元数据**：时间戳、token 使用量和执行指标

## 🎨 高级功能

### 交互模式

在交互模式下，你可以：

- 输入任何任务描述来执行
- 使用 `status` 查看智能体信息
- 使用 `help` 获取可用命令
- 使用 `clear` 清屏
- 使用 `exit` 或 `quit` 结束会话

### 多提供商支持

```bash
# 使用 OpenAI
sage run "创建 Python 脚本" --provider openai --model gpt-4

# 使用 Anthropic
sage run "代码审查" --provider anthropic --model claude-3-5-sonnet

# 使用自定义工作目录
sage run "添加单元测试" --working-dir /path/to/project
```

### 配置优先级

1. 命令行参数（最高优先级）
2. 配置文件值
3. 环境变量
4. 默认值（最低优先级）

## ⚡ 性能优化

### 最佳实践

- **并发处理**：Sage Agent 使用 Tokio 异步运行时，支持高效的并发操作
- **内存管理**：Rust 的零成本抽象确保最小的运行时开销
- **缓存策略**：智能缓存 LLM 响应和工具结果以提高性能
- **流式处理**：支持流式 LLM 响应以获得更好的用户体验

### 配置调优

```json
{
  "model_parameters": {
    "temperature": 0.1,        // 降低随机性以获得更一致的结果
    "max_tokens": 2000,        // 根据任务复杂度调整
    "stream": true             // 启用流式响应
  },
  "max_steps": 15,             // 限制最大步数以控制成本
  "timeout_seconds": 300       // 设置合理的超时时间
}
```

### 监控和日志

```bash
# 启用详细日志
RUST_LOG=sage_core=debug,sage_cli=info cargo run

# 监控 token 使用
sage run "任务描述" --show-stats

# 性能分析
RUST_LOG=trace cargo run --release
```

## 🔧 开发

### 构建

```bash
# 构建所有 crate
cargo build

# 优化构建
cargo build --release

# 运行测试
cargo test

# 带日志运行
RUST_LOG=debug cargo run
```

### 项目结构

```
sage-agent/
├── crates/
│   ├── sage-core/          # 核心库
│   │   ├── src/agent/      # 智能体执行逻辑
│   │   ├── src/llm/        # LLM 客户端实现
│   │   ├── src/tools/      # 工具系统
│   │   └── src/ui/         # 终端 UI 组件
│   ├── sage-cli/           # 命令行界面
│   ├── sage-sdk/           # 高级 SDK
│   └── sage-tools/         # 内置工具集合
├── examples/               # 使用示例
├── trajectories/           # 执行轨迹文件（已忽略）
├── configs/                # 配置模板和示例
└── Cargo.toml             # 工作空间配置
```

## 🎯 使用场景

- **代码生成**：创建文件、函数和整个模块
- **代码分析**：理解和记录现有代码库
- **重构**：现代化和改进代码结构
- **测试**：生成和运行测试套件
- **文档**：创建全面的项目文档
- **自动化**：自动化重复的开发任务

## 📝 配置

Sage Agent 通过 JSON 文件和环境变量支持灵活配置：

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

## 📚 文档

完整的文档可在 [`docs/`](docs/) 目录中找到：

- **[用户指南](docs/user-guide/)** - 安装、配置和使用说明
- **[开发指南](docs/development/)** - 贡献和开发环境设置
- **[架构文档](docs/architecture/)** - 系统设计和架构
- **[API 参考](docs/api/)** - 详细的 API 文档
- **[规划和路线图](docs/planning/)** - 项目路线图和 TODO 列表

### 快速链接
- [入门指南](docs/user-guide/getting-started.md) - 新用户指南
- [贡献指南](docs/development/contributing.md) - 如何贡献
- [TODO 列表](docs/planning/) - 当前开发优先级
- [MCP 集成计划](docs/development/MCP_INTEGRATION_PLAN.md) - 模型上下文协议支持
- [文档一致性指南](docs/DOC_CONSISTENCY_GUIDE.md) - 维护文档一致性

## 🔧 故障排除

### 常见问题

**导入错误：**
```bash
# 尝试设置 RUST_LOG
RUST_LOG=debug cargo run
```

**API 密钥问题：**
```bash
# 验证 API 密钥是否设置
echo $OPENAI_API_KEY
echo $ANTHROPIC_API_KEY

# 检查配置
sage --show-config
```

**权限错误：**
```bash
# 确保文件操作有适当权限
chmod +x /path/to/your/project
```

### 环境变量

- `OPENAI_API_KEY` - OpenAI API 密钥
- `ANTHROPIC_API_KEY` - Anthropic API 密钥
- `GOOGLE_API_KEY` - Google Gemini API 密钥
- `OPENROUTER_API_KEY` - OpenRouter API 密钥

### 开发指南

- 遵循 Rust 官方代码风格指南
- 为新功能添加测试
- 根据需要更新文档
- 适当使用类型提示
- 提交前确保所有测试通过

## 🔄 迁移见解

在开发过程中，我们对 Rust 与 TypeScript 在 AI 智能体开发方面进行了广泛的对比分析。以下是我们的主要发现：

### 🎯 性能分析

| 方面 | Rust | TypeScript | 胜者 |
|------|------|------------|------|
| **并发工具执行** | ~120ms | ~150ms | Rust (+25%) |
| **LLM API 调用** | 1-5 秒 | 1-5 秒 | 平局 |
| **整体智能体响应** | 1.12-5.15s | 1.15-5.15s | **差异可忽略** |

**关键洞察**：在 AI 智能体场景中，网络 I/O 占主导地位，语言选择的影响较小。

### 🛠️ 开发体验

| 因素 | Rust | TypeScript |
|------|------|------------|
| **生态系统丰富度** | AI 库有限 | 丰富的 AI/LLM 生态系统 |
| **开发速度** | 较慢（编译时间） | 较快（热重载） |
| **团队入门** | 学习曲线陡峭 | 大多数开发者熟悉 |
| **调试** | 复杂（异步 + FFI） | 直观 |
| **部署** | 复杂（跨平台） | 简单（Node.js） |

### 🏗️ 架构经验

**Rust 中表现良好的方面：**
- ✅ **并发工具执行**：优秀的 async/await 模式
- ✅ **类型安全**：编译时保证防止了许多错误
- ✅ **内存效率**：零成本抽象
- ✅ **清晰架构**：强制良好的设计模式

**具有挑战性的方面：**
- ❌ **UI 集成**：终端 UI 的复杂 FFI
- ❌ **生态系统缺口**：缺少 AI 特定库
- ❌ **构建复杂性**：跨平台编译问题
- ❌ **开发速度**：较慢的迭代周期

### 📊 推荐方法

对于 **AI 智能体开发**，我们推荐：

1. **TypeScript/Node.js** 用于快速原型和丰富的 AI 生态系统
2. **Rust** 用于性能关键组件（如果需要）
3. **混合方法** 以获得两者的优势

### 🎓 学习价值

此 Rust 实现展示了：
- 高级并发编程模式
- 系统编程中的清晰架构
- 现代异步 Rust 技术
- 使用 React + Ink 的终端 UI 开发

## 🤝 贡献

**注意**：虽然此项目已归档，但我们欢迎讨论和学习交流！请查看我们的[贡献指南](docs/development/contributing.md)了解历史背景：

- [开发环境设置](docs/development/setup.md)
- [代码风格和约定](docs/development/code-style.md)
- [测试要求](docs/development/testing.md)
- [拉取请求流程](docs/development/contributing.md#pull-requests)

## 📄 许可证

本项目采用 MIT 许可证 - 详情请参阅 [LICENSE](LICENSE) 文件。

**注意**：此 Rust 实现与原始 [Trae Agent](https://github.com/bytedance/trae-agent) 项目的 MIT 许可证保持兼容。

## 🙏 致谢

- **原始灵感**：本项目基于字节跳动的 [Trae Agent](https://github.com/bytedance/trae-agent) - 一个开创性的基于 LLM 的软件工程任务智能体
- **部分灵感来源**：[Augment Code](https://www.augmentcode.com/) - 先进的AI代码助手和上下文引擎，为智能体工具系统设计提供了宝贵的参考
- **架构见解**：[Gemini CLI](https://github.com/google-gemini/gemini-cli) - 基于 TypeScript 的 AI 智能体架构的优秀参考
- 使用 [Rust](https://rust-lang.org/) 和现代异步模式构建
- 由领先的 LLM 提供商（Google、Anthropic、OpenAI 等）提供支持
- 受开源社区对智能开发自动化承诺的启发
- 特别感谢 Trae Agent 贡献者和维护者的基础工作
- 感谢 Augment Code 团队在AI辅助开发领域的创新工作
- 感谢 Rust 社区提供的优秀异步编程模式和工具

### 🎓 教育价值

此归档项目作为以下方面的综合示例：
- **现代 Rust 开发** 与 async/await 模式
- **AI 智能体系统中的并发工具执行**
- **使用 React + Ink 集成的终端 UI 开发**
- **系统编程中的清晰架构** 原则
- **LLM 集成** 模式和最佳实践

---

**Sage Agent** - AI 智能体架构的学习之旅。📚✨

*"每个归档的项目都为下一个项目教会我们宝贵的东西。"*
