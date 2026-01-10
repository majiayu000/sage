# Sage Slash Command 架构重构方案

## 三个项目对比分析

基于对 Sage、Open Claude Code 和 Crush 三个项目的深入分析，本文档总结最佳实践并提出 Sage 的重构方案。

---

## 架构对比表

| 特性 | Sage (当前) | Open Claude Code | Crush |
|-----|------------|------------------|-------|
| **触发方式** | 文本解析 `/command` | 文本解析 `/command` | Dialog 触发 (`/` on empty) |
| **命令类型** | 2 类 (Local + Expandable) | 2 类 (Local-JSX + Prompt) | 3 类 (System + User + MCP) |
| **处理层数** | 2 层 (硬编码 + Registry) | 3 层 (UI + Processing + Permission) | 4 层 (UI + Dialog + Loading + Execution) |
| **用户自定义** | .sage/commands/*.md | ~/.claude-code/skills/*.md | ~/.crush/commands/*.md |
| **参数支持** | 有限 | `$ARGUMENTS` | `$PARAM_NAME` |
| **权限系统** | 无 | 5 级 (Policy→User→Project→Skill→Tool) | 无 |
| **工具限制** | 无 | `allowed-tools` | 无 |
| **MCP 集成** | 部分 | 有 | 完整 |
| **分类 UI** | 无 | 无 | Radio 选择器 |

---

## 设计模式对比

### 命令定义方式

| 项目 | 方式 | 优点 | 缺点 |
|-----|------|------|------|
| Sage | Rust trait + 硬编码 | 类型安全 | 扩展需改代码 |
| Open Claude Code | Markdown + YAML | 易编写 | 元数据有限 |
| Crush | Markdown + 模板变量 | 简单直观 | 无元数据 |

### 命令触发方式

| 项目 | 方式 | 优点 | 缺点 |
|-----|------|------|------|
| Sage | 字符串前缀匹配 | 简单 | 不可扩展 |
| Open Claude Code | 命令解析器 | 灵活 | 复杂 |
| Crush | Dialog 选择器 | UX 好 | 需额外点击 |

### 命令执行方式

| 项目 | 方式 | 优点 | 缺点 |
|-----|------|------|------|
| Sage | 多处处理器 | 无 | 混乱、重复 |
| Open Claude Code | 统一执行器 + 权限 | 安全 | 复杂 |
| Crush | Message-driven | 解耦 | 间接 |

---

## 最佳实践总结

### 从 Open Claude Code 学习

1. **Skill/Prompt 系统** - Markdown + YAML frontmatter
2. **多级权限** - 至少支持 User/Project 级配置
3. **工具限制** - 命令可限制可用工具
4. **模型选择** - 命令可指定模型

### 从 Crush 学习

1. **三分类系统** - System/User/MCP 清晰分离
2. **Dialog UI** - 优雅的命令选择体验
3. **模板变量** - `$PARAM_NAME` 参数注入
4. **目录发现** - 约定优于配置的命令加载

### 需要避免的问题

1. **Sage 当前问题** - 多处处理器、死代码、混合机制
2. **Open Claude Code 问题** - 代码混淆、文件过大
3. **Crush 问题** - 静默错误、无缓存

---

## Sage 重构方案

### 目标架构

```
用户输入
    │
    ▼
┌─────────────────────────────────────────────────────────┐
│              CommandRouter (唯一入口)                   │
│  - 判断是否为命令 (is_command)                          │
│  - 路由到对应处理器                                     │
└──────────────────────┬──────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│ System Cmd  │ │  User Cmd   │ │  MCP Cmd    │
│ (内置命令)   │ │ (Markdown)  │ │ (MCP Prompt)│
└──────┬──────┘ └──────┬──────┘ └──────┬──────┘
       │               │               │
       └───────────────┼───────────────┘
                       ▼
┌─────────────────────────────────────────────────────────┐
│              CommandResult                              │
│  - Local: 直接输出                                      │
│  - Prompt: 发送给 LLM                                   │
│  - Interactive: 需要 CLI 处理                           │
└─────────────────────────────────────────────────────────┘
```

### 新目录结构

```
sage-core/src/commands/
├── mod.rs                    # 模块导出
├── router.rs                 # 命令路由器 (新增)
├── types.rs                  # CommandResult, CommandType
├── registry/
│   ├── mod.rs               # CommandRegistry
│   ├── system.rs            # 系统命令注册 (重命名自 builtins.rs)
│   ├── user.rs              # 用户命令加载 (新增)
│   └── mcp.rs               # MCP 命令加载 (新增)
└── executor/
    ├── mod.rs               # CommandExecutor
    └── handlers/
        ├── mod.rs           # 命令分发
        ├── system.rs        # 系统命令处理 (重命名)
        └── interactive.rs   # 交互命令处理 (新增)

~/.sage/
├── commands/                 # 用户自定义命令
│   ├── my-command.md
│   └── review/
│       └── pr.md
└── config.json
```

### 命令文件格式

```markdown
---
name: "Review PR"
description: "Review a pull request"
allowed_tools: ["Read", "Grep", "Bash"]
model: "sonnet"
---

Review the following pull request: $PR_URL

Focus on:
1. Code quality
2. Security issues
3. Performance concerns

$ADDITIONAL_CONTEXT
```

### 核心数据结构

```rust
/// 命令类型
#[derive(Clone, Debug, PartialEq)]
pub enum CommandType {
    System,  // 内置命令
    User,    // 用户 Markdown 命令
    Mcp,     // MCP Prompt
}

/// 命令定义
#[derive(Clone, Debug)]
pub struct CommandDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command_type: CommandType,
    pub shortcut: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub template: Option<String>,
}

/// 命令执行结果
#[derive(Clone, Debug)]
pub struct CommandResult {
    pub kind: CommandResultKind,
    pub source_command: String,
}

#[derive(Clone, Debug)]
pub enum CommandResultKind {
    /// 本地处理完成，直接输出
    Local { output: String },

    /// 发送给 LLM 的 prompt
    Prompt {
        content: String,
        tool_restrictions: Option<Vec<String>>,
        model_override: Option<String>,
    },

    /// 需要 CLI 交互处理
    Interactive(InteractiveCommand),

    /// 需要参数输入
    NeedsArguments {
        command: CommandDefinition,
        arguments: Vec<ArgumentDefinition>,
    },
}

/// 参数定义
#[derive(Clone, Debug)]
pub struct ArgumentDefinition {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default_value: Option<String>,
}
```

### 命令路由器

```rust
/// 统一命令入口
pub struct CommandRouter {
    system_commands: HashMap<String, CommandDefinition>,
    user_commands: Vec<CommandDefinition>,
    mcp_commands: Vec<CommandDefinition>,
}

impl CommandRouter {
    /// 判断是否为命令
    pub fn is_command(input: &str) -> bool {
        input.starts_with('/')
    }

    /// 处理命令
    pub async fn route(&self, input: &str) -> SageResult<CommandResult> {
        let (cmd_name, args) = Self::parse_command(input);

        // 1. 系统命令优先
        if let Some(cmd) = self.system_commands.get(&cmd_name) {
            return self.execute_system(cmd, args).await;
        }

        // 2. 用户命令
        if let Some(cmd) = self.find_user_command(&cmd_name) {
            return self.execute_user(cmd, args).await;
        }

        // 3. MCP 命令
        if let Some(cmd) = self.find_mcp_command(&cmd_name) {
            return self.execute_mcp(cmd, args).await;
        }

        Err(SageError::command(format!("Unknown command: {}", cmd_name)))
    }
}
```

---

## 实现 TodoList

### Phase 1: 清理当前代码 (Day 1)

- [ ] 删除 `slash_commands.rs` 未使用代码
- [ ] 删除 `run/execution.rs` 未使用代码
- [ ] 删除 `unified.rs` 中的 `handle_interactive_command` (L1282)
- [ ] 统一硬编码命令到 Registry

### Phase 2: 重构命令系统 (Day 2-3)

- [ ] 创建 `router.rs` - 统一命令入口
- [ ] 重构 `types.rs` - 新数据结构
- [ ] 重命名 `builtins.rs` → `system.rs`
- [ ] 实现系统命令注册

### Phase 3: 用户命令支持 (Day 4-5)

- [ ] 创建 `registry/user.rs` - 用户命令加载
- [ ] 实现 Markdown + YAML frontmatter 解析
- [ ] 实现 `$PARAM` 变量提取
- [ ] 实现参数输入 UI

### Phase 4: MCP 集成 (Day 6)

- [ ] 创建 `registry/mcp.rs` - MCP 命令加载
- [ ] 集成 MCP Prompt 系统
- [ ] 实现 MCP 参数处理

### Phase 5: UI 改进 (Day 7)

- [ ] 添加命令分类选择器
- [ ] 改进命令帮助显示
- [ ] 添加命令搜索/过滤

---

## 系统命令清单

| 命令 | 功能 | 类型 | 说明 |
|-----|------|------|------|
| `/help` | 显示帮助 | Local | 显示可用命令 |
| `/exit`, `/quit` | 退出 | Local | 退出程序 |
| `/clear` | 清屏 | Local | 清除对话 |
| `/login` | 登录配置 | Interactive | 配置 API Key |
| `/logout` | 清除凭证 | Local | 删除凭证文件 |
| `/resume` | 恢复会话 | Interactive | 选择并恢复会话 |
| `/title` | 设置标题 | Interactive | 设置会话标题 |
| `/model` | 切换模型 | Interactive | 选择 LLM 模型 |
| `/commit` | Git 提交 | Prompt | 智能提交助手 |
| `/review` | 代码审查 | Prompt | 代码审查 |
| `/explain` | 解释代码 | Prompt | 代码解释 |
| `/test` | 生成测试 | Prompt | 测试生成 |

---

## 迁移路径

### 向后兼容

**不需要**。根据项目规范，直接迁移即可。

### 迁移步骤

1. **删除死代码** - 清理未使用的处理器
2. **统一入口** - 所有命令通过 `CommandRouter`
3. **重新注册** - 系统命令迁移到新 Registry
4. **添加扩展** - 用户命令、MCP 支持

---

## 验证计划

### 单元测试

```rust
#[test]
fn test_command_router_system() {
    let router = CommandRouter::new();
    let result = router.route("/help").await;
    assert!(matches!(result, Ok(CommandResult { kind: CommandResultKind::Local { .. }, .. })));
}

#[test]
fn test_user_command_loading() {
    let commands = UserCommandLoader::load_from_dir("~/.sage/commands");
    assert!(!commands.is_empty());
}

#[test]
fn test_param_extraction() {
    let template = "Review $PR_URL with focus on $FOCUS_AREA";
    let params = extract_params(template);
    assert_eq!(params, vec!["PR_URL", "FOCUS_AREA"]);
}
```

### 集成测试

```bash
# 1. 测试系统命令
sage /help
sage /login
sage /resume

# 2. 测试用户命令
echo "---\nname: test\n---\nHello \$NAME" > ~/.sage/commands/test.md
sage /test NAME=World

# 3. 测试命令发现
sage /commands  # 列出所有可用命令
```

---

## 结论

通过学习 Open Claude Code 的 Skill 系统和 Crush 的三分类架构，Sage 应该：

1. **统一命令入口** - 消除多处处理器的混乱
2. **支持三类命令** - System/User/MCP
3. **Markdown 模板** - 易于编写用户命令
4. **参数系统** - `$PARAM` 变量和参数 UI
5. **清理死代码** - 删除未使用的处理器

这将使 Sage 的 slash command 系统更加清晰、可扩展、易维护。
