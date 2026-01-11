---
name: tink-ui-migration
description: Sage 到 Tink UI 框架的完全重构指南，包含迁移步骤、文件清单、架构设计
when_to_use: 当进行 Tink UI 重构、修改 UI 相关代码、添加新 UI 组件时使用
allowed_tools:
  - Read
  - Edit
  - Write
  - Bash
  - Glob
  - Grep
user_invocable: true
priority: 100
---

# Sage Tink UI 迁移指南

## 一、迁移目标

将 Sage CLI 的 UI 层从当前的 `colored` + `print!` 命令式方案迁移到 Tink 声明式 UI 框架。

**核心变化**:
- 命令式 `print!()` → 声明式 `rsx! {}`
- 手动布局计算 → Flexbox 自动布局
- 分散的颜色调用 → 统一主题系统
- 多个动画实现 → Tink Spinner 组件

## 二、当前架构问题

| 问题 | 影响文件数 | 说明 |
|------|-----------|------|
| print!/println! 分散 | 82+ | UI 输出逻辑分散在整个代码库 |
| colored 直接使用 | 15+ | 没有统一的颜色抽象层 |
| 图标系统重复 | 2 | sage-core 和 sage-cli 各有一套 |
| 控制台抽象不统一 | 3 | CliConsole、NerdConsole、EnhancedConsole 重叠 |
| 动画实现重复 | 2 | AnimationManager 和 ThinkingSpinner 功能重叠 |

## 三、需要重构的文件清单

### Phase 1: 核心 UI 层 (sage-core/src/ui/)

| 文件 | 行数 | 处理方式 | 优先级 |
|------|-----|---------|-------|
| `ui/mod.rs` | 23 | 重写，导出 Tink 组件 | P0 |
| `ui/animation.rs` | 320 | **删除**，用 Tink Spinner 替代 | P0 |
| `ui/display.rs` | 205 | **删除**，用 Tink 组件替代 | P0 |
| `ui/progress.rs` | 399 | **删除**，用 Tink Progress 替代 | P0 |
| `ui/prompt.rs` | 354 | 重写为 Tink 交互组件 | P1 |
| `ui/icons.rs` | 248 | 保留，集成到 Tink Text | P1 |
| `ui/markdown.rs` | 339 | 保留，适配 Tink Text | P2 |
| `ui/claude_style.rs` | 119 | **删除**，合并到新组件 | P0 |
| `ui/enhanced_console.rs` | 385 | **删除**，用 Tink 组件替代 | P0 |

### Phase 2: 输出策略层 (sage-core/src/output/)

| 文件 | 行数 | 处理方式 | 优先级 |
|------|-----|---------|-------|
| `output/mod.rs` | 78 | 重写，适配 Tink 渲染模型 | P0 |
| `output/strategy.rs` | 402 | **完全重写**，改为 Tink 状态更新 | P0 |
| `output/formatter.rs` | 456 | 保留 JSON 格式化，删除 Text | P1 |

### Phase 3: CLI UI 层 (sage-cli/src/ui/)

| 文件 | 行数 | 处理方式 | 优先级 |
|------|-----|---------|-------|
| `ui/mod.rs` | 16 | 重写 | P0 |
| `ui/nerd_console.rs` | 334 | **删除**，合并到 Tink 应用 | P0 |
| `ui/components.rs` | 288 | **删除**，用 Tink 组件替代 | P0 |
| `ui/icons.rs` | 235 | **删除**，合并到 sage-core | P0 |

### Phase 4: CLI 控制台 (sage-cli/src/)

| 文件 | 行数 | 处理方式 | 优先级 |
|------|-----|---------|-------|
| `console.rs` | 403 | **删除**，用 Tink 应用替代 | P0 |
| `progress.rs` | 179 | **删除**，用 Tink 组件替代 | P0 |

### Phase 5: 命令处理 (sage-cli/src/commands/)

| 文件 | 处理方式 | 优先级 |
|------|---------|-------|
| `unified/input.rs` | 重写为 Tink use_input | P1 |
| `unified/interactive.rs` | 重写为 Tink 应用 | P0 |
| `unified/slash_commands.rs` | 适配新 UI | P1 |
| `unified/outcome.rs` | 适配新 UI | P1 |
| `interactive/onboarding.rs` | 重写为 Tink 向导 | P2 |
| `diagnostics.rs` | 适配新 UI | P2 |
| `config.rs` | 适配新 UI | P2 |
| `tools.rs` | 适配新 UI | P2 |
| `trajectory.rs` | 适配新 UI | P2 |

### Phase 6: Agent 内部显示

| 文件 | 处理方式 | 优先级 |
|------|---------|-------|
| `agent/unified/tool_display.rs` | 重写，通过 signal 更新状态 | P0 |
| `agent/unified/llm_orchestrator.rs` | 适配流式输出到 Tink | P0 |
| `agent/unified/user_interaction.rs` | 适配 Tink 交互 | P1 |
| `agent/unified/event_manager/mod.rs` | 改为发送 Tink 状态更新 | P0 |

## 四、新架构设计

### 4.1 目录结构

```
crates/
├── sage-core/
│   └── src/
│       ├── ui/                    # Tink 组件库
│       │   ├── mod.rs            # 导出所有组件
│       │   ├── app.rs            # 主应用组件
│       │   ├── components/       # UI 组件
│       │   │   ├── mod.rs
│       │   │   ├── message.rs    # 消息显示组件
│       │   │   ├── tool_call.rs  # 工具调用显示
│       │   │   ├── thinking.rs   # 思考状态组件
│       │   │   ├── input.rs      # 输入框组件
│       │   │   └── status.rs     # 状态栏组件
│       │   ├── theme.rs          # 主题系统
│       │   ├── icons.rs          # 图标 (保留)
│       │   └── markdown.rs       # Markdown 渲染 (适配)
│       └── output/
│           ├── mod.rs
│           ├── state.rs          # 应用状态定义
│           └── events.rs         # 状态更新事件
│
├── sage-cli/
│   └── src/
│       ├── main.rs
│       ├── app.rs                # Tink 应用入口
│       └── commands/
│           └── ...               # 命令处理 (简化)
```

### 4.2 核心状态模型

```rust
use tink::prelude::*;

/// 应用全局状态
#[derive(Clone)]
pub struct AppState {
    /// 当前阶段
    pub phase: Phase,
    /// 消息列表
    pub messages: Vec<Message>,
    /// 当前输入
    pub input: String,
    /// 工具执行状态
    pub tool_state: Option<ToolState>,
    /// 思考状态
    pub thinking: Option<ThinkingState>,
}

#[derive(Clone)]
pub enum Phase {
    Idle,
    Thinking,
    Streaming,
    ExecutingTool,
    WaitingInput,
}

#[derive(Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ToolState {
    pub name: String,
    pub description: String,
    pub status: ToolStatus,
}

#[derive(Clone)]
pub struct ThinkingState {
    pub start_time: Instant,
    pub completed: bool,
    pub duration: Option<Duration>,
}
```

### 4.3 主应用组件

```rust
use tink::prelude::*;

fn SageApp() -> Element {
    let state = use_signal(AppState::default);

    // 监听 Agent 事件
    use_effect(move || {
        // 订阅执行事件，更新状态
    });

    rsx! {
        Box { flex_direction: FlexDirection::Column, height: Size::full(),
            // 消息列表
            MessageList { messages: state.get().messages.clone() }

            // 当前状态显示
            StatusBar { phase: state.get().phase.clone() }

            // 输入框
            InputBox {
                value: state.get().input.clone(),
                on_submit: move |text| { /* 处理提交 */ }
            }
        }
    }
}
```

### 4.4 消息组件

```rust
fn MessageList(messages: Vec<Message>) -> Element {
    rsx! {
        Box { flex_direction: FlexDirection::Column, flex_grow: 1.0,
            for msg in messages {
                MessageItem { message: msg }
            }
        }
    }
}

fn MessageItem(message: Message) -> Element {
    let (icon, color) = match message.role {
        Role::User => ("❯", Color::Blue),
        Role::Assistant => ("●", Color::White),
    };

    rsx! {
        Box { flex_direction: FlexDirection::Row,
            Text { color: color, bold: true, "{icon} " }
            Text { "{message.content}" }
        }
    }
}
```

### 4.5 工具执行组件

```rust
fn ToolExecution(tool: ToolState) -> Element {
    rsx! {
        Box { padding_left: 2,
            match tool.status {
                ToolStatus::Running => rsx! {
                    Spinner { style: SpinnerStyle::Dots }
                    Text { color: Color::Green, " Running · {tool.description}" }
                },
                ToolStatus::Completed => rsx! {
                    Text { color: Color::Green, "✓ " }
                    Text { dimmed: true, "{tool.name}" }
                },
                ToolStatus::Failed(ref err) => rsx! {
                    Text { color: Color::Red, "✗ " }
                    Text { "{tool.name}: {err}" }
                },
            }
        }
    }
}
```

### 4.6 思考状态组件

```rust
fn ThinkingIndicator(state: ThinkingState) -> Element {
    if state.completed {
        rsx! {
            Text { dimmed: true, "  ✓ Thought for {state.duration:?}" }
        }
    } else {
        let elapsed = use_signal(|| Instant::now().duration_since(state.start_time));

        use_interval(Duration::from_millis(100), move || {
            elapsed.set(Instant::now().duration_since(state.start_time));
        });

        rsx! {
            Spinner { style: SpinnerStyle::Dots }
            Text { dimmed: true, " Thinking ({elapsed.get().as_secs_f32():.1}s)" }
        }
    }
}
```

## 五、迁移步骤

### Step 1: 添加 Tink 依赖

```toml
# Cargo.toml
[workspace.dependencies]
tink = { path = "../../../tool/tink" }

# sage-core/Cargo.toml
[dependencies]
tink = { workspace = true }

# sage-cli/Cargo.toml
[dependencies]
tink = { workspace = true }
```

### Step 2: 创建状态模型

1. 创建 `sage-core/src/ui/state.rs`
2. 定义 AppState、Phase、Message 等类型
3. 实现状态更新方法

### Step 3: 创建基础组件

1. 创建 `sage-core/src/ui/components/` 目录
2. 实现 MessageItem、ToolExecution、ThinkingIndicator
3. 实现主题系统 (颜色常量)

### Step 4: 适配 Agent 事件

1. 修改 `event_manager` 发送状态更新而非直接打印
2. 修改 `tool_display.rs` 更新 ToolState
3. 修改 `llm_orchestrator.rs` 更新流式内容

### Step 5: 重写 CLI 入口

1. 修改 `sage-cli/src/main.rs` 启动 Tink 应用
2. 创建 `sage-cli/src/app.rs` 主应用组件
3. 连接 Agent 和 UI 状态

### Step 6: 删除旧代码

1. 删除 `sage-core/src/ui/` 中的旧文件
2. 删除 `sage-cli/src/ui/` 中的旧文件
3. 删除 `sage-cli/src/console.rs`、`progress.rs`
4. 移除 `colored`、`indicatif`、`dialoguer` 依赖

### Step 7: 适配命令

1. 重写 `interactive.rs` 使用 Tink 应用
2. 适配 `slash_commands.rs`
3. 适配其他命令的输出

## 六、要删除的依赖

迁移完成后移除:

```toml
# 从 Cargo.toml 删除
colored = "2"
indicatif = "0.17"
dialoguer = "0.11"
console = "0.15"
```

保留:
```toml
syntect = "5"          # Markdown 语法高亮仍需要
pulldown-cmark = "0.9" # Markdown 解析仍需要
textwrap = "0.16"      # Tink 内部使用
```

## 七、测试策略

### 单元测试
- 测试状态更新逻辑
- 测试组件渲染输出

### 集成测试
- 测试 Agent 事件 → UI 状态更新流程
- 测试流式输出性能

### 手动测试
- 验证动画流畅度
- 验证键盘输入响应
- 验证终端大小变化处理

## 八、风险和缓解

| 风险 | 缓解措施 |
|------|---------|
| Tink 性能问题 | 先做性能基准测试 |
| 流式输出延迟 | 实现增量渲染优化 |
| 终端兼容性 | 保留 ASCII 回退模式 |
| 调试困难 | 添加详细日志 |

## 九、版本计划

| 版本 | 里程碑 |
|------|-------|
| 0.1.x | 当前版本，继续 bug 修复 |
| 0.2.0 | Tink 集成基础框架 |
| 0.2.x | 逐步迁移各模块 |
| 0.3.0 | 完成迁移，删除旧代码 |
| 1.0.0 | 稳定版本发布 |

## 十、注意事项

1. **不需要向后兼容**: 可以大胆删除旧代码
2. **逐步迁移**: 一次只迁移一个模块
3. **保持可运行**: 每次提交后确保 `cargo build` 通过
4. **版本递增**: 每次有意义的更新递增 0.1.x
