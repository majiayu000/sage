# Contributing to Sage Agent | 贡献指南

[English](#english) | [中文](#中文)

---

<a name="english"></a>
## English

### Welcome Contributors

Thank you for your interest in contributing to **Sage Agent**! This project is a Rust-based LLM agent system inspired by Claude Code's design patterns. We welcome contributions of all kinds - bug fixes, new features, documentation improvements, and more.

Whether you're fixing a typo or implementing a major feature, your contribution is valued and appreciated.

---

### Development Environment Setup

#### Prerequisites

**Rust Requirements:**
- Rust 1.85+ (Rust 2024 Edition)
- `rustfmt` and `clippy` components
- Basic familiarity with async/await and Tokio

**System Requirements:**
- Git
- Unix-like environment (macOS, Linux) or WSL on Windows
- Terminal with UTF-8 support

#### Installation Steps

1. **Clone the repository**
   ```bash
   git clone https://github.com/majiayu000/sage.git
   cd sage
   ```

2. **Setup Rust toolchain**
   ```bash
   # Update Rust to latest stable
   rustup update

   # Install required components
   rustup component add rustfmt clippy
   ```

3. **Verify installation**
   ```bash
   # Check Rust version
   rustc --version

   # Run development setup
   make setup
   ```

4. **Build the project**
   ```bash
   # Debug build
   make build
   # or
   cargo build

   # Release build
   make release
   ```

5. **Run tests**
   ```bash
   # All tests
   make test

   # Unit tests only
   make test-unit

   # Integration tests
   make test-int
   ```

6. **Configure API keys** (for development)
   ```bash
   # Copy example configuration
   cp sage_config.json.example sage_config.json

   # Set environment variables
   export ANTHROPIC_API_KEY="your-key-here"
   export OPENAI_API_KEY="your-key-here"
   ```

---

### Code Standards

#### Formatting

**Always format code before committing:**

```bash
# Format all code
make fmt
# or
cargo fmt

# Check formatting without modifying
cargo fmt -- --check
```

**Settings:**
- Use `rustfmt.toml` in project root
- 100-character line limit
- 4-space indentation
- No trailing whitespace

#### Linting

**Run Clippy before submitting PR:**

```bash
# Run clippy with warnings as errors
make clippy
# or
cargo clippy -- -D warnings

# Auto-fix some warnings
cargo clippy --fix
```

**Standards:**
- Fix all Clippy warnings
- No `#[allow(clippy::...)]` without justification
- Prefer idiomatic Rust patterns
- Document `unsafe` code blocks

#### Testing Requirements

**All code changes must include tests:**

1. **Unit Tests** (required)
   - Test individual functions/methods
   - Mock external dependencies
   - Aim for >80% coverage

2. **Integration Tests** (for new features)
   - Test tool integration
   - Test agent workflows
   - Test error handling

3. **Documentation Tests** (for public APIs)
   - Include examples in doc comments
   - Ensure examples compile and run

**Testing Guidelines:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = "test";

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_feature() {
        // Test async code
    }
}
```

**Run tests:**
```bash
# All tests
cargo test

# Specific test
cargo test test_feature_name

# With output
cargo test -- --nocapture

# Run examples as tests
make examples
```

---

### Commit Standards

We follow **Conventional Commits** specification.

#### Commit Message Format

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

#### Commit Types

| Type | Description | Example |
|------|-------------|---------|
| `feat` | New feature | `feat(tools): add WebSearch tool` |
| `fix` | Bug fix | `fix(agent): resolve infinite loop in retry logic` |
| `docs` | Documentation only | `docs: update CONTRIBUTING.md` |
| `style` | Code style (formatting, semicolons, etc.) | `style: run cargo fmt` |
| `refactor` | Code refactoring | `refactor(llm): simplify provider factory` |
| `perf` | Performance improvement | `perf(tools): optimize glob search` |
| `test` | Adding/updating tests | `test(core): add integration tests for agent` |
| `chore` | Build process, dependencies | `chore: update tokio to 1.35` |
| `ci` | CI/CD changes | `ci: add clippy check to workflow` |

#### Scopes

Common scopes in this project:
- `core` - sage-core crate
- `cli` - sage-cli crate
- `sdk` - sage-sdk crate
- `tools` - sage-tools crate
- `agent` - agent execution logic
- `llm` - LLM providers
- `ui` - terminal UI components
- `session` - session management
- `commands` - slash commands

#### Examples

**Good commit messages:**
```bash
feat(tools): implement Bash tool with background execution support

- Add run_in_background parameter
- Support timeout configuration
- Add process cleanup on cancellation

Closes #123

---

fix(llm): handle rate limiting for Anthropic API

The client now implements exponential backoff with jitter
when receiving 429 responses.

---

docs(readme): add SDK usage examples

Added three examples:
- Basic usage
- Non-interactive execution
- Custom configuration

---

refactor(agent): extract state machine into separate module

This improves testability and separation of concerns.
No functional changes.
```

**Bad commit messages:**
```bash
# ❌ Too vague
fix: bug fix

# ❌ Missing scope
add new tool

# ❌ Not descriptive
update code

# ❌ Multiple changes in one commit
feat: add WebSearch tool, fix bash timeout, update docs
```

---

### Pull Request Process

#### Before Opening PR

1. **Create a feature branch**
   ```bash
   # Branch naming format: type/short-description
   git checkout -b feat/add-websearch-tool
   git checkout -b fix/agent-infinite-loop
   git checkout -b docs/update-contributing
   ```

2. **Make your changes**
   ```bash
   # Write code, add tests, update docs
   ```

3. **Run quality checks**
   ```bash
   # Quick check (formatting + linting + tests)
   make quick

   # Full CI check
   make ci
   ```

4. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat(tools): add WebSearch tool"
   ```

5. **Push to your fork**
   ```bash
   git push origin feat/add-websearch-tool
   ```

#### PR Title Format

Follow the same format as commit messages:

```
<type>(<scope>): <description>
```

Examples:
- `feat(tools): add WebSearch tool with domain filtering`
- `fix(agent): resolve memory leak in trajectory recording`
- `docs: add contribution guidelines`

#### PR Description Template

When you open a PR, include:

```markdown
## Summary
Brief description of what this PR does.

## Changes
- List key changes
- Be specific about what was modified
- Mention any breaking changes

## Motivation
Why is this change needed? What problem does it solve?

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Documentation updated

## Screenshots (if applicable)
For UI changes, include before/after screenshots.

## Related Issues
Closes #123
Relates to #456

## Checklist
- [ ] Code follows project style guidelines
- [ ] Ran `make quick` successfully
- [ ] Added tests for new functionality
- [ ] Updated documentation
- [ ] Commit messages follow Conventional Commits
```

#### Code Review Process

1. **Automated Checks**
   - CI must pass (build, tests, clippy, fmt)
   - All tests must pass
   - No clippy warnings

2. **Maintainer Review**
   - Code quality and style
   - Test coverage
   - Documentation completeness
   - Architecture alignment

3. **Addressing Feedback**
   ```bash
   # Make requested changes
   git add .
   git commit -m "refactor: address review feedback"
   git push origin feat/your-branch
   ```

4. **Merge**
   - Squash merge for single commits
   - Rebase merge for clean multi-commit history
   - Delete branch after merge

---

### Project Structure

Understanding the codebase organization:

```
sage/
├── crates/
│   ├── sage-core/          # Core library (agent engine, LLM, tools)
│   │   ├── src/
│   │   │   ├── agent/      # Agent execution logic
│   │   │   │   ├── base.rs           # Base agent trait
│   │   │   │   ├── execution.rs      # Main execution loop
│   │   │   │   ├── state.rs          # State management
│   │   │   │   └── unified.rs        # Unified agent mode
│   │   │   ├── llm/        # LLM provider implementations
│   │   │   │   ├── anthropic.rs      # Anthropic/Claude
│   │   │   │   ├── openai.rs         # OpenAI
│   │   │   │   ├── google.rs         # Google Gemini
│   │   │   │   └── factory.rs        # Provider factory
│   │   │   ├── commands/   # Slash command system
│   │   │   ├── session/    # Session management
│   │   │   ├── tools/      # Tool registry
│   │   │   └── ui/         # Terminal UI components
│   │   └── Cargo.toml
│   │
│   ├── sage-cli/           # Command-line interface
│   │   ├── src/
│   │   │   ├── main.rs     # CLI entry point
│   │   │   └── commands/   # CLI subcommands
│   │   └── Cargo.toml
│   │
│   ├── sage-sdk/           # High-level SDK for programmatic use
│   │   ├── src/
│   │   │   └── client.rs   # SDK client
│   │   └── Cargo.toml
│   │
│   └── sage-tools/         # Built-in tool implementations
│       ├── src/
│       │   └── tools/
│       │       ├── file_ops/      # File operations (Read, Write, Edit, Glob, Grep)
│       │       ├── bash.rs        # Bash command execution
│       │       ├── web_search.rs  # Web search
│       │       └── task_mgmt/     # Task management
│       └── Cargo.toml
│
├── examples/               # Usage examples
├── docs/                   # Documentation
│   ├── user-guide/        # User documentation
│   ├── development/       # Developer guides
│   ├── architecture/      # System design
│   └── api/               # API reference
├── configs/               # Configuration templates
└── Makefile              # Build automation
```

#### Key Crates

| Crate | Purpose | Key Dependencies |
|-------|---------|------------------|
| **sage-core** | Core agent engine | tokio, anyhow, serde, tracing |
| **sage-cli** | Command-line interface | clap, indicatif, crossterm |
| **sage-sdk** | High-level SDK | sage-core |
| **sage-tools** | Built-in tools | ignore, regex, reqwest |

---

### Common Tasks

#### 1. Adding a New Tool

**Steps:**

1. Create tool implementation in `crates/sage-tools/src/tools/`

```rust
// crates/sage-tools/src/tools/my_new_tool.rs
use async_trait::async_trait;
use sage_core::tools::{Tool, ToolError, ToolResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MyNewToolParams {
    pub input: String,
}

pub struct MyNewTool;

#[async_trait]
impl Tool for MyNewTool {
    type Params = MyNewToolParams;

    fn name(&self) -> &str {
        "my_new_tool"
    }

    fn description(&self) -> &str {
        "Description of what this tool does"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input parameter description"
                }
            },
            "required": ["input"]
        })
    }

    async fn execute(&self, params: Self::Params) -> ToolResult {
        // Implementation here
        Ok(format!("Processed: {}", params.input))
    }
}
```

2. Register tool in `crates/sage-tools/src/lib.rs`

```rust
pub mod tools {
    pub mod my_new_tool;
}

use tools::my_new_tool::MyNewTool;

pub fn register_default_tools(registry: &mut ToolRegistry) {
    registry.register(Box::new(MyNewTool));
    // ... other tools
}
```

3. Add tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_new_tool() {
        let tool = MyNewTool;
        let params = MyNewToolParams {
            input: "test".to_string(),
        };
        let result = tool.execute(params).await;
        assert!(result.is_ok());
    }
}
```

4. Add documentation in `docs/tools/my_new_tool.md`

#### 2. Adding a New LLM Provider

**Steps:**

1. Create provider implementation in `crates/sage-core/src/llm/`

```rust
// crates/sage-core/src/llm/my_provider.rs
use async_trait::async_trait;
use crate::llm::{LLMClient, LLMRequest, LLMResponse, LLMError};

pub struct MyProviderClient {
    api_key: String,
    model: String,
    base_url: String,
}

impl MyProviderClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: "https://api.myprovider.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMClient for MyProviderClient {
    async fn chat(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        // Implementation here
        todo!()
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
```

2. Add provider to factory in `crates/sage-core/src/llm/factory.rs`

```rust
pub enum ProviderType {
    OpenAI,
    Anthropic,
    MyProvider,  // Add your provider
}

pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn LLMClient>> {
    match config.provider_type {
        ProviderType::MyProvider => {
            Ok(Box::new(MyProviderClient::new(
                config.api_key.clone(),
                config.model.clone(),
            )))
        }
        // ... other providers
    }
}
```

3. Update configuration schema in `crates/sage-core/src/config.rs`

4. Add integration tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_provider_chat() {
        let client = MyProviderClient::new(
            "test-key".to_string(),
            "test-model".to_string(),
        );
        // Test implementation
    }
}
```

5. Update documentation:
   - `README.md` - Add to providers table
   - `docs/user-guide/configuration.md` - Add config example
   - Create `docs/providers/my_provider.md`

#### 3. Adding Tests

**Unit Test Example:**

```rust
// In the same file as your code
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        let result = function_to_test(input);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

**Integration Test Example:**

```rust
// In crates/sage-core/tests/integration_test.rs
use sage_core::agent::Agent;
use sage_core::config::Config;

#[tokio::test]
async fn test_agent_execution() {
    let config = Config::default();
    let agent = Agent::new(config);

    let result = agent.run("test task").await;
    assert!(result.is_ok());
}
```

**Tool Integration Test:**

```rust
// In crates/sage-tools/tests/my_tool_integration.rs
use sage_tools::tools::my_new_tool::MyNewTool;
use sage_core::tools::Tool;

#[tokio::test]
async fn test_tool_integration() {
    let tool = MyNewTool;
    let params = /* params */;
    let result = tool.execute(params).await;
    assert!(result.is_ok());
}
```

---

### Getting Help

- **Documentation**: Check `docs/` directory
- **Examples**: Run examples in `examples/` directory
- **Issues**: Search existing issues or create new one
- **Discussions**: Ask questions in GitHub Discussions

### Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others in the community
- Report unacceptable behavior

---

<a name="中文"></a>
## 中文

### 欢迎贡献者

感谢您对 **Sage Agent** 项目的关注！这是一个基于 Rust 的 LLM 智能体系统，设计灵感来自 Claude Code。我们欢迎各种形式的贡献 - bug 修复、新功能、文档改进等。

无论您是修复一个拼写错误还是实现一个重要功能，您的贡献都是宝贵的。

---

### 开发环境设置

#### 前置要求

**Rust 要求：**
- Rust 1.85+ (Rust 2024 Edition)
- `rustfmt` 和 `clippy` 组件
- 基本的 async/await 和 Tokio 知识

**系统要求：**
- Git
- 类 Unix 环境（macOS、Linux）或 Windows 上的 WSL
- 支持 UTF-8 的终端

#### 安装步骤

1. **克隆仓库**
   ```bash
   git clone https://github.com/majiayu000/sage.git
   cd sage
   ```

2. **设置 Rust 工具链**
   ```bash
   # 更新 Rust 到最新稳定版
   rustup update

   # 安装必需组件
   rustup component add rustfmt clippy
   ```

3. **验证安装**
   ```bash
   # 检查 Rust 版本
   rustc --version

   # 运行开发环境设置
   make setup
   ```

4. **构建项目**
   ```bash
   # Debug 构建
   make build
   # 或
   cargo build

   # Release 构建
   make release
   ```

5. **运行测试**
   ```bash
   # 所有测试
   make test

   # 仅单元测试
   make test-unit

   # 集成测试
   make test-int
   ```

6. **配置 API 密钥**（用于开发）
   ```bash
   # 复制示例配置
   cp sage_config.json.example sage_config.json

   # 设置环境变量
   export ANTHROPIC_API_KEY="your-key-here"
   export OPENAI_API_KEY="your-key-here"
   ```

---

### 代码规范

#### 代码格式化

**提交前务必格式化代码：**

```bash
# 格式化所有代码
make fmt
# 或
cargo fmt

# 检查格式而不修改
cargo fmt -- --check
```

**规范：**
- 使用项目根目录的 `rustfmt.toml`
- 100 字符行宽限制
- 4 空格缩进
- 无尾随空格

#### 代码检查（Linting）

**提交 PR 前运行 Clippy：**

```bash
# 运行 clippy，将警告视为错误
make clippy
# 或
cargo clippy -- -D warnings

# 自动修复部分警告
cargo clippy --fix
```

**标准：**
- 修复所有 Clippy 警告
- 除非有充分理由，否则不使用 `#[allow(clippy::...)]`
- 优先使用惯用的 Rust 模式
- 记录 `unsafe` 代码块

#### 测试要求

**所有代码更改必须包含测试：**

1. **单元测试**（必需）
   - 测试单个函数/方法
   - Mock 外部依赖
   - 力争 >80% 覆盖率

2. **集成测试**（新功能需要）
   - 测试工具集成
   - 测试 Agent 工作流
   - 测试错误处理

3. **文档测试**（公共 API 需要）
   - 在文档注释中包含示例
   - 确保示例可编译和运行

**测试指南：**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange - 准备
        let input = "test";

        // Act - 执行
        let result = function_under_test(input);

        // Assert - 断言
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_feature() {
        // 测试异步代码
    }
}
```

**运行测试：**
```bash
# 所有测试
cargo test

# 特定测试
cargo test test_feature_name

# 显示输出
cargo test -- --nocapture

# 运行示例作为测试
make examples
```

---

### 提交规范

我们遵循 **Conventional Commits** 规范。

#### 提交消息格式

```
<type>(<scope>): <subject>

[可选的 body]

[可选的 footer]
```

#### 提交类型

| 类型 | 描述 | 示例 |
|------|-----|------|
| `feat` | 新功能 | `feat(tools): add WebSearch tool` |
| `fix` | Bug 修复 | `fix(agent): resolve infinite loop in retry logic` |
| `docs` | 仅文档 | `docs: update CONTRIBUTING.md` |
| `style` | 代码风格（格式化、分号等） | `style: run cargo fmt` |
| `refactor` | 代码重构 | `refactor(llm): simplify provider factory` |
| `perf` | 性能优化 | `perf(tools): optimize glob search` |
| `test` | 添加/更新测试 | `test(core): add integration tests for agent` |
| `chore` | 构建过程、依赖 | `chore: update tokio to 1.35` |
| `ci` | CI/CD 变更 | `ci: add clippy check to workflow` |

#### 作用域（Scope）

项目中常见的作用域：
- `core` - sage-core crate
- `cli` - sage-cli crate
- `sdk` - sage-sdk crate
- `tools` - sage-tools crate
- `agent` - Agent 执行逻辑
- `llm` - LLM 提供商
- `ui` - 终端 UI 组件
- `session` - 会话管理
- `commands` - 斜杠命令

#### 示例

**好的提交消息：**
```bash
feat(tools): implement Bash tool with background execution support

- Add run_in_background parameter
- Support timeout configuration
- Add process cleanup on cancellation

Closes #123

---

fix(llm): handle rate limiting for Anthropic API

The client now implements exponential backoff with jitter
when receiving 429 responses.

---

docs(readme): add SDK usage examples

Added three examples:
- Basic usage
- Non-interactive execution
- Custom configuration

---

refactor(agent): extract state machine into separate module

This improves testability and separation of concerns.
No functional changes.
```

**不好的提交消息：**
```bash
# ❌ 太模糊
fix: bug fix

# ❌ 缺少 scope
add new tool

# ❌ 不够描述性
update code

# ❌ 一个提交包含多个变更
feat: add WebSearch tool, fix bash timeout, update docs
```

---

### Pull Request 流程

#### 开启 PR 之前

1. **创建功能分支**
   ```bash
   # 分支命名格式：type/short-description
   git checkout -b feat/add-websearch-tool
   git checkout -b fix/agent-infinite-loop
   git checkout -b docs/update-contributing
   ```

2. **进行更改**
   ```bash
   # 编写代码、添加测试、更新文档
   ```

3. **运行质量检查**
   ```bash
   # 快速检查（格式化 + lint + 测试）
   make quick

   # 完整 CI 检查
   make ci
   ```

4. **提交更改**
   ```bash
   git add .
   git commit -m "feat(tools): add WebSearch tool"
   ```

5. **推送到您的 fork**
   ```bash
   git push origin feat/add-websearch-tool
   ```

#### PR 标题格式

遵循与提交消息相同的格式：

```
<type>(<scope>): <description>
```

示例：
- `feat(tools): add WebSearch tool with domain filtering`
- `fix(agent): resolve memory leak in trajectory recording`
- `docs: add contribution guidelines`

#### PR 描述模板

开启 PR 时，请包含：

```markdown
## 概述
简要描述此 PR 的作用。

## 变更内容
- 列出关键变更
- 具体说明修改了什么
- 提及任何破坏性变更

## 动机
为什么需要这个变更？解决了什么问题？

## 测试
- [ ] 添加/更新了单元测试
- [ ] 添加/更新了集成测试
- [ ] 执行了手动测试
- [ ] 更新了文档

## 截图（如适用）
对于 UI 变更，包含变更前后的截图。

## 相关 Issue
Closes #123
Relates to #456

## 检查清单
- [ ] 代码遵循项目风格指南
- [ ] 成功运行 `make quick`
- [ ] 为新功能添加了测试
- [ ] 更新了文档
- [ ] 提交消息遵循 Conventional Commits
```

#### 代码审查流程

1. **自动检查**
   - CI 必须通过（构建、测试、clippy、fmt）
   - 所有测试必须通过
   - 无 clippy 警告

2. **维护者审查**
   - 代码质量和风格
   - 测试覆盖率
   - 文档完整性
   - 架构对齐

3. **处理反馈**
   ```bash
   # 进行请求的更改
   git add .
   git commit -m "refactor: address review feedback"
   git push origin feat/your-branch
   ```

4. **合并**
   - 单个提交使用 Squash merge
   - 干净的多提交历史使用 Rebase merge
   - 合并后删除分支

---

### 项目结构

理解代码库组织：

```
sage/
├── crates/
│   ├── sage-core/          # 核心库（Agent 引擎、LLM、工具）
│   │   ├── src/
│   │   │   ├── agent/      # Agent 执行逻辑
│   │   │   │   ├── base.rs           # 基础 Agent trait
│   │   │   │   ├── execution.rs      # 主执行循环
│   │   │   │   ├── state.rs          # 状态管理
│   │   │   │   └── unified.rs        # 统一 Agent 模式
│   │   │   ├── llm/        # LLM 提供商实现
│   │   │   │   ├── anthropic.rs      # Anthropic/Claude
│   │   │   │   ├── openai.rs         # OpenAI
│   │   │   │   ├── google.rs         # Google Gemini
│   │   │   │   └── factory.rs        # 提供商工厂
│   │   │   ├── commands/   # 斜杠命令系统
│   │   │   ├── session/    # 会话管理
│   │   │   ├── tools/      # 工具注册表
│   │   │   └── ui/         # 终端 UI 组件
│   │   └── Cargo.toml
│   │
│   ├── sage-cli/           # 命令行界面
│   │   ├── src/
│   │   │   ├── main.rs     # CLI 入口
│   │   │   └── commands/   # CLI 子命令
│   │   └── Cargo.toml
│   │
│   ├── sage-sdk/           # 高级 SDK（编程使用）
│   │   ├── src/
│   │   │   └── client.rs   # SDK 客户端
│   │   └── Cargo.toml
│   │
│   └── sage-tools/         # 内置工具实现
│       ├── src/
│       │   └── tools/
│       │       ├── file_ops/      # 文件操作（Read、Write、Edit、Glob、Grep）
│       │       ├── bash.rs        # Bash 命令执行
│       │       ├── web_search.rs  # Web 搜索
│       │       └── task_mgmt/     # 任务管理
│       └── Cargo.toml
│
├── examples/               # 使用示例
├── docs/                   # 文档
│   ├── user-guide/        # 用户文档
│   ├── development/       # 开发者指南
│   ├── architecture/      # 系统设计
│   └── api/               # API 参考
├── configs/               # 配置模板
└── Makefile              # 构建自动化
```

#### 核心 Crate

| Crate | 用途 | 主要依赖 |
|-------|------|---------|
| **sage-core** | 核心 Agent 引擎 | tokio, anyhow, serde, tracing |
| **sage-cli** | 命令行界面 | clap, indicatif, crossterm |
| **sage-sdk** | 高级 SDK | sage-core |
| **sage-tools** | 内置工具 | ignore, regex, reqwest |

---

### 常见任务

#### 1. 添加新工具

**步骤：**

1. 在 `crates/sage-tools/src/tools/` 创建工具实现

```rust
// crates/sage-tools/src/tools/my_new_tool.rs
use async_trait::async_trait;
use sage_core::tools::{Tool, ToolError, ToolResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MyNewToolParams {
    pub input: String,
}

pub struct MyNewTool;

#[async_trait]
impl Tool for MyNewTool {
    type Params = MyNewToolParams;

    fn name(&self) -> &str {
        "my_new_tool"
    }

    fn description(&self) -> &str {
        "此工具的功能描述"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "输入参数描述"
                }
            },
            "required": ["input"]
        })
    }

    async fn execute(&self, params: Self::Params) -> ToolResult {
        // 在此实现
        Ok(format!("已处理: {}", params.input))
    }
}
```

2. 在 `crates/sage-tools/src/lib.rs` 注册工具

```rust
pub mod tools {
    pub mod my_new_tool;
}

use tools::my_new_tool::MyNewTool;

pub fn register_default_tools(registry: &mut ToolRegistry) {
    registry.register(Box::new(MyNewTool));
    // ... 其他工具
}
```

3. 添加测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_new_tool() {
        let tool = MyNewTool;
        let params = MyNewToolParams {
            input: "test".to_string(),
        };
        let result = tool.execute(params).await;
        assert!(result.is_ok());
    }
}
```

4. 在 `docs/tools/my_new_tool.md` 添加文档

#### 2. 添加新 LLM 提供商

**步骤：**

1. 在 `crates/sage-core/src/llm/` 创建提供商实现

```rust
// crates/sage-core/src/llm/my_provider.rs
use async_trait::async_trait;
use crate::llm::{LLMClient, LLMRequest, LLMResponse, LLMError};

pub struct MyProviderClient {
    api_key: String,
    model: String,
    base_url: String,
}

impl MyProviderClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: "https://api.myprovider.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMClient for MyProviderClient {
    async fn chat(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        // 在此实现
        todo!()
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
```

2. 在 `crates/sage-core/src/llm/factory.rs` 添加提供商到工厂

```rust
pub enum ProviderType {
    OpenAI,
    Anthropic,
    MyProvider,  // 添加您的提供商
}

pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn LLMClient>> {
    match config.provider_type {
        ProviderType::MyProvider => {
            Ok(Box::new(MyProviderClient::new(
                config.api_key.clone(),
                config.model.clone(),
            )))
        }
        // ... 其他提供商
    }
}
```

3. 在 `crates/sage-core/src/config.rs` 更新配置 schema

4. 添加集成测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_provider_chat() {
        let client = MyProviderClient::new(
            "test-key".to_string(),
            "test-model".to_string(),
        );
        // 测试实现
    }
}
```

5. 更新文档：
   - `README.md` - 添加到提供商表格
   - `docs/user-guide/configuration.md` - 添加配置示例
   - 创建 `docs/providers/my_provider.md`

#### 3. 添加测试

**单元测试示例：**

```rust
// 在代码同文件中
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        let result = function_to_test(input);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

**集成测试示例：**

```rust
// 在 crates/sage-core/tests/integration_test.rs
use sage_core::agent::Agent;
use sage_core::config::Config;

#[tokio::test]
async fn test_agent_execution() {
    let config = Config::default();
    let agent = Agent::new(config);

    let result = agent.run("test task").await;
    assert!(result.is_ok());
}
```

**工具集成测试示例：**

```rust
// 在 crates/sage-tools/tests/my_tool_integration.rs
use sage_tools::tools::my_new_tool::MyNewTool;
use sage_core::tools::Tool;

#[tokio::test]
async fn test_tool_integration() {
    let tool = MyNewTool;
    let params = /* params */;
    let result = tool.execute(params).await;
    assert!(result.is_ok());
}
```

---

### 获取帮助

- **文档**：查看 `docs/` 目录
- **示例**：运行 `examples/` 目录中的示例
- **Issue**：搜索现有 issue 或创建新 issue
- **讨论**：在 GitHub Discussions 中提问

### 行为准则

- 尊重和包容他人
- 专注于建设性反馈
- 帮助社区中的其他人
- 报告不可接受的行为

---

## License | 许可证

By contributing to Sage Agent, you agree that your contributions will be licensed under the MIT License.

通过为 Sage Agent 做出贡献，您同意您的贡献将根据 MIT 许可证进行许可。

---

**Thank you for contributing to Sage Agent!** | **感谢您为 Sage Agent 做出贡献！**
