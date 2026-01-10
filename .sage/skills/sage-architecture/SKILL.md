---
name: sage-architecture
description: Sage 项目整体架构设计指南，基于 Claude Code、Crush 最佳实践的融合方案
when_to_use: 当涉及 Sage 项目架构设计、模块规划、重构决策时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 100
---

# Sage 项目架构指南

## 核心设计原则

### 1. 模块边界原则

**每个模块 < 200 行**，超过则拆分为子模块。

```
正确 ✓
src/tools/
├── mod.rs           # 公开接口 (~50行)
├── registry.rs      # 注册表 (~150行)
├── executor.rs      # 执行器 (~180行)
└── permission/      # 子模块
    ├── mod.rs
    ├── rules.rs
    └── handlers.rs

错误 ✗
src/tools/
├── mod.rs           # 800行的巨型文件
```

### 2. 层级职责分离

```
┌─────────────────────────────────────────────────────────────┐
│                      sage-cli (用户界面)                      │
├─────────────────────────────────────────────────────────────┤
│                      sage-sdk (高级 API)                      │
├─────────────────────────────────────────────────────────────┤
│                     sage-core (核心引擎)                       │
│  ┌─────────┬─────────┬─────────┬─────────┬─────────┐       │
│  │  agent  │   llm   │  tools  │ skills  │ context │       │
│  ├─────────┼─────────┼─────────┼─────────┼─────────┤       │
│  │recovery │learning │checkpoint│  mcp   │ storage │       │
│  └─────────┴─────────┴─────────┴─────────┴─────────┘       │
├─────────────────────────────────────────────────────────────┤
│                    sage-tools (工具实现)                      │
└─────────────────────────────────────────────────────────────┘
```

### 3. Sage 独有优势（必须保持）

| 模块 | 功能 | 竞品对比 |
|-----|------|---------|
| `recovery/` | 熔断器、限流器、supervisor | OpenClaude/Crush 无 |
| `learning/` | 模式学习、用户偏好 | OpenClaude/Crush 无 |
| `checkpoints/` | 状态快照、回滚恢复 | OpenClaude/Crush 无 |
| `storage/` | 多后端(SQLite/Postgres) | Crush 仅 SQLite |
| `sandbox/` | 安全沙箱执行 | OpenClaude/Crush 无 |

**绝不删除这些模块，它们是 Sage 的核心竞争力。**

### 4. 学习 Crush 的简洁模式

**Prompt 模板化**（而非硬编码）：
```
prompts/
├── templates/
│   ├── coder.md.tpl      # 主 system prompt
│   ├── task.md.tpl       # 任务 prompt
│   └── initialize.md.tpl # 初始化 prompt
├── renderer.rs           # 模板渲染器
└── variables.rs          # 变量定义
```

**工具描述文件**（TOOL.md）：
```
tools/builtin/
├── bash/
│   ├── mod.rs      # 实现
│   └── TOOL.md     # 描述（注入 system prompt）
├── edit/
│   ├── mod.rs
│   └── TOOL.md
```

## 模块重组方案

### 需要合并的模块

| 当前 | 合并到 | 原因 |
|-----|-------|------|
| `settings/` | `config/` | 功能重叠 |
| `session/` | `context/` | 会话是上下文的一部分 |
| `sandbox/` | `tools/sandbox/` | 沙箱服务于工具执行 |
| `modes/` | `agent/modes/` | 模式是 agent 的状态 |
| `cost/` | `telemetry/cost/` | 成本是遥测的一部分 |

### 需要拆分的模块

| 当前 | 拆分为 | 原因 |
|-----|-------|------|
| `llm/` 过大 | `llm/{client,providers,streaming,fallback}/` | 文件超 200 行 |
| `agent/unified/` | `agent/executor/{loop,step,completion}/` | 职责过多 |

## 命名规范

### 类型命名（RFC 430）

```rust
// 正确 ✓ - 缩写词当作普通单词
LlmClient, SseDecoder, McpServer, HttpRequest

// 错误 ✗ - 全大写缩写
LLMClient, SSEDecoder, MCPServer, HTTPRequest
```

### 模块命名

```rust
// 正确 ✓ - 小写下划线
pub mod rate_limiter;
pub mod circuit_breaker;

// 错误 ✗ - 驼峰或连字符
pub mod rateLimiter;
pub mod circuit-breaker;
```

## 依赖关系规则

```
sage-cli → sage-sdk → sage-core → sage-tools
                ↓
           外部 crates (tokio, serde, etc.)
```

**禁止**：
- `sage-core` 依赖 `sage-cli`
- `sage-tools` 依赖 `sage-sdk`
- 循环依赖

## 新模块检查清单

创建新模块前确认：

- [ ] 是否真的需要新模块？能否放入现有模块？
- [ ] 模块职责单一吗？
- [ ] 预计代码量 < 200 行吗？
- [ ] 与现有模块的边界清晰吗？
- [ ] 命名符合 RFC 430 吗？
- [ ] 有对应的测试模块吗？

## 文件结构模板

```rust
//! 模块简短描述
//!
//! 详细说明（可选）

mod submodule1;
mod submodule2;

pub use submodule1::PublicType;
pub use submodule2::public_function;

// 私有实现（如果有）
```
