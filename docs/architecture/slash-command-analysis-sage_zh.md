# Sage Slash Command 架构分析

## 当前状态：混乱

### 问题概述

Sage 的 slash command 系统存在严重的架构混乱，有多个重复的处理器和两套混合使用的处理机制。

---

## 架构图

```
用户输入 "/command"
       │
       ▼
┌─────────────────────────────────────────────────┐
│           execute_interactive_loop()             │
│  (crates/sage-cli/src/commands/unified.rs)       │
├─────────────────────────────────────────────────┤
│                                                  │
│  ┌─ 第一层：硬编码字符串检查 ─────────────────┐  │
│  │                                            │  │
│  │  if input == "/exit" ...                   │  │
│  │  if input == "/clear" ...                  │  │
│  │  if input == "/help" ...                   │  │
│  │  if input == "/login" ...    ← 新增       │  │
│  │  if input == "/logout" ...   ← 新增       │  │
│  │                                            │  │
│  │  问题：手动添加，不可扩展                   │  │
│  └────────────────────────────────────────────┘  │
│                      │                           │
│                      ▼ (如果没匹配)              │
│  ┌─ 第二层：CommandRegistry 处理 ─────────────┐  │
│  │                                            │  │
│  │  process_slash_command()                   │  │
│  │       │                                    │  │
│  │       ▼                                    │  │
│  │  CommandExecutor::process()                │  │
│  │       │                                    │  │
│  │       ▼                                    │  │
│  │  ┌─────────────────────────────────────┐   │  │
│  │  │  CommandRegistry (sage-core)        │   │  │
│  │  │  - builtins.rs 注册命令             │   │  │
│  │  │  - handlers/ 执行命令               │   │  │
│  │  └─────────────────────────────────────┘   │  │
│  │       │                                    │  │
│  │       ▼                                    │  │
│  │  返回 CommandResult                        │  │
│  │    - is_local: bool                       │  │
│  │    - interactive: Option<InteractiveCommand>│ │
│  │    - expanded_prompt: String              │  │
│  │                                            │  │
│  └────────────────────────────────────────────┘  │
│                      │                           │
│                      ▼                           │
│  ┌─ 第三层：InteractiveCommand 处理 ──────────┐  │
│  │                                            │  │
│  │  handle_interactive_command_v2()           │  │
│  │  (第 634-670 行)                           │  │
│  │                                            │  │
│  │  问题：还有 3 个未使用的同名函数！         │  │
│  └────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

---

## 命令分类

### 1. 本地命令 (Local Commands)
直接在 CLI 层处理，不需要 LLM 参与。

| 命令 | 处理位置 | 说明 |
|-----|---------|------|
| `/exit`, `/quit` | 硬编码 (L345) | 退出程序 |
| `/clear` | 硬编码 (L351) | 清屏 |
| `/help` | 硬编码 (L362) | 显示帮助 |
| `/login` | 硬编码 (L368) + Registry | 配置凭证 |
| `/logout` | 硬编码 (L388) | 清除凭证 |
| `/resume` | Registry → InteractiveCommand | 恢复会话 |
| `/title` | Registry → InteractiveCommand | 设置标题 |

### 2. 扩展命令 (Expandable Commands)
通过 CommandRegistry 注册，返回扩展后的 prompt 发给 LLM。

| 命令 | 定义位置 | 说明 |
|-----|---------|------|
| `/commit` | builtins.rs | Git 提交 |
| `/review` | builtins.rs | 代码审查 |
| `/explain` | builtins.rs | 解释代码 |
| `/test` | builtins.rs | 生成测试 |
| 自定义 | .sage/commands/*.md | 用户自定义 |

---

## 文件结构

```
sage-core/src/commands/
├── mod.rs                  # 模块导出
├── types.rs                # CommandResult, InteractiveCommand 定义
├── executor/
│   ├── mod.rs              # CommandExecutor 实现
│   └── handlers/
│       ├── mod.rs          # 命令分发
│       ├── basic.rs        # 基础命令 (help, version)
│       └── advanced.rs     # 高级命令 (login, resume, title)
└── registry/
    ├── mod.rs              # CommandRegistry 实现
    ├── builtins.rs         # 内置命令注册
    └── discovery.rs        # 自定义命令发现

sage-cli/src/commands/
├── unified.rs              # 主执行循环 (包含硬编码命令)
├── interactive/
│   ├── mod.rs              # 交互模式 (大部分未使用)
│   ├── slash_commands.rs   # ⚠️ 未使用的处理器
│   ├── onboarding.rs       # 登录向导实现
│   └── ...
└── run/
    ├── execution.rs        # ⚠️ 未使用的处理器
    └── ...
```

---

## 重复处理器清单

### InteractiveCommand::Login 处理

| 位置 | 函数名 | 状态 | 行为 |
|-----|-------|------|------|
| unified.rs:368 | if 检查 | ✅ 使用 | 直接处理 |
| unified.rs:649 | handle_interactive_command_v2 | ✅ 使用 | 运行登录 |
| unified.rs:1297 | handle_interactive_command | ❌ 未使用 | 显示错误 |
| slash_commands.rs:91 | handle_interactive_command | ❌ 未使用 | 调用 handler |
| slash_commands.rs:98 | handle_login_command | ❌ 未使用 | 运行登录 |
| run/execution.rs:304 | handle_interactive_command | ❌ 未使用 | 显示错误 |

### 核心问题

1. **6 个地方处理同一个命令**
2. **3 个同名函数** (`handle_interactive_command`)
3. **2 套混合使用的机制**（硬编码 + Registry）

---

## 数据流分析

### InteractiveCommand 枚举

```rust
// sage-core/src/commands/types.rs
pub enum InteractiveCommand {
    Resume { session_id: Option<String>, show_all: bool },
    Title { title: String },
    Login,
}
```

### CommandResult 结构

```rust
pub struct CommandResult {
    pub expanded_prompt: String,      // 扩展后的 prompt
    pub is_local: bool,               // 是否本地处理
    pub interactive: Option<InteractiveCommand>, // 需要 CLI 处理的命令
    pub status_message: Option<String>,
    pub local_output: Option<String>,
    pub show_expansion: bool,
}
```

---

## 问题总结

### 1. 架构不一致
- 本地命令有的用硬编码，有的用 Registry
- 没有统一的命令处理入口

### 2. 死代码堆积
- `slash_commands.rs` 整个文件未使用
- `run/execution.rs` 大部分函数未使用
- `unified.rs` 中有未使用的函数

### 3. 可扩展性差
- 添加新命令需要在多处修改
- 没有清晰的命令生命周期

### 4. 关注点分离不清
- sage-core 和 sage-cli 职责边界模糊
- InteractiveCommand 的处理分散在多处

---

## 期望目标

1. **单一入口**：所有命令通过同一套机制处理
2. **清晰分层**：sage-core 负责解析，sage-cli 负责执行
3. **易于扩展**：添加新命令只需一处修改
4. **无死代码**：删除所有未使用的处理器
