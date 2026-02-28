# Sage UI Specification

本文档定义 Sage CLI 的用户界面规范，包括：
- 显示架构
- 交互规范
- 状态反馈
- 已知问题及解决方案

## 1. UI 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                      Terminal Display                        │
├─────────────────────────────────────────────────────────────┤
│  sage-cli/src/ui/           │  sage-core/src/ui/            │
│  ├── icons.rs (Nerd Font)   │  ├── display.rs (DisplayMgr)  │
│  ├── nerd_console.rs        │  ├── markdown.rs              │
│  └── mod.rs                 │  ├── animation.rs             │
│                             │  └── enhanced_console.rs      │
├─────────────────────────────────────────────────────────────┤
│  Display Layers:                                             │
│  1. Header (session info)                                    │
│  2. Chat Area (messages, tool calls)                        │
│  3. Status Line (thinking, executing)                       │
│  4. Input Prompt                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.1 文件职责

| 文件 | 职责 |
|------|------|
| `sage-cli/src/ui/icons.rs` | Nerd Font 图标常量，ASCII fallback |
| `sage-cli/src/ui/nerd_console.rs` | CLI 层 UI 组件（header, prompt, sessions） |
| `sage-core/src/ui/display.rs` | 核心显示管理器，markdown 渲染 |
| `sage-core/src/ui/animation.rs` | 加载动画（Thinking, Executing） |
| `sage-core/src/agent/unified/step_execution.rs` | 执行时的实时输出 |

## 2. 显示状态与格式

### 2.1 会话启动

```
  󰚩 sage   main   ~/project   claude-sonnet-4
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

   Recent Sessions

  ├──  Fix login bug (2h ago, 12 msgs)
  └──  Add dark mode (1d ago, 8 msgs)

  ℹ Type your message, or /help for commands. Press Ctrl+C to exit.

  sage ❯ _
```

**规范：**
- Header 使用 2 空格缩进
- 分隔线 `━` 填满终端宽度
- Session tree 使用 `├──` 和 `└──` 连接符

### 2.2 AI 响应

```
  󰚩 AI Response

  这是 AI 的回复内容。Markdown 格式会被渲染：
  - 列表项 1
  - 列表项 2

  ```rust
  fn example() {
      println!("代码高亮");
  }
  ```
```

**规范：**
- Header: `  󰚩 AI Response` (2空格 + 图标 + 文字)
- 内容: 每行 2 空格缩进
- Markdown 正常渲染

### 2.3 工具执行

```
   Executing tools (3)

   bash git status
    ✓ done (45ms)

   read /path/to/file.rs
    ✓ done (12ms)

   write /path/to/output.rs
    ✗ failed (8ms)
      Permission denied: /path/to/output.rs
```

**规范：**
- Header: `   Executing tools (N)` 显示工具总数
- 每个工具:
  - `  {icon} {name} {params}` - 图标 + 名称 + 关键参数
  - `    ✓ done (Nms)` 或 `    ✗ failed (Nms)` - 结果状态
  - 失败时显示错误首行（缩进 6 空格）

**工具图标映射：**

| 工具 | 图标 |
|------|------|
| bash, shell, execute |  |
| read, cat |  |
| write, edit |  |
| grep, search |  |
| glob, find |  |
| lsp, code |  |
| web_fetch, web_search | 󰖟 |
| task, todo_write |  |
| 其他 |  |

### 2.4 思考状态

```
   Thinking...
```

**规范：**
- 使用动画（spinner）
- 显示在当前行，不换行
- 完成后清除该行

### 2.5 执行摘要

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✓ Completed

  ⚡ 5 steps   󰊤 1234 in / 567 out   󰔛 2.3s
```

**规范：**
- 分隔线在上方
- 状态: `✓ Completed` (绿色) 或 `✗ Failed` (红色)
- 统计: steps, tokens in/out, duration

## 3. 交互规范

### 3.1 输入提示

```
  sage ❯ _
```

**规范：**
- 2 空格缩进
- `sage` 青色加粗
- `❯` 青色
- 光标位置在 ❯ 后面一个空格

### 3.2 中文输入处理

- 退格键：ASCII 字符宽度 = 1，CJK 字符宽度 = 2
- Ctrl+U：清除整行
- Enter：提交输入

### 3.3 斜杠命令

| 命令 | 描述 | 交互方式 |
|------|------|----------|
| `/help` | 显示帮助 | 立即显示，不需确认 |
| `/clear` | 清屏 | 立即执行，重绘 header |
| `/resume` | 恢复会话 | **交互式选择**（方向键） |
| `/cost` | 显示费用 | 立即显示 |
| `/exit` | 退出 | 立即退出 |

### 3.4 权限确认对话框

```
╭────────────────────────────────────────────────────╮
│  Permission Required                               │
├────────────────────────────────────────────────────┤
│  Tool: bash                                        │
│  Command: rm -rf ./temp                            │
│                                                    │
│  This is a destructive operation.                 │
│                                                    │
│  [Y] Yes  [N] No  [A] Always  [V] Never           │
╰────────────────────────────────────────────────────╯
```

## 4. 已知问题及解决方案

### 4.1 问题：开放式任务看起来"卡住"

**现象：**
- Agent 执行对比分析等开放式任务
- 不断调用工具探索代码（10+次）
- 可能启动 subagent
- 长时间没有最终结论输出
- 用户无法判断是在工作还是卡住

**根因：**
1. 缺少整体进度指示
2. 没有显示当前在做什么（探索 vs 分析 vs 总结）
3. Subagent 执行时主线程无反馈
4. 没有预估完成时间或阶段

**解决方案：**

#### 4.1.1 阶段指示器

```
  󰚩 sage  Phase 1/3: Exploring codebase

   Executing tools (5)
   ...
```

显示当前处于哪个阶段：
1. Exploring - 探索/搜索
2. Analyzing - 分析/比较
3. Synthesizing - 综合/总结

#### 4.1.2 Subagent 状态显示

```
  󰜗 Subagent running: "Analyze auth module"
    └──  Step 3/10  read auth/mod.rs
```

当启动 subagent 时：
- 显示 subagent 任务描述
- 显示 subagent 当前步骤
- 保持主线程响应

#### 4.1.3 长时间任务进度条

```
  ⏳ Long-running task (2m 30s elapsed)
  [████████████░░░░░░░░] 60% - Reading files
```

当任务超过 30 秒时显示：
- 已用时间
- 进度估算（基于 step 数量）
- 当前活动描述

#### 4.1.4 活动指示器（Heartbeat）

```
  ● Working... (last activity: read file 3s ago)
```

每 5 秒更新一次，显示：
- Agent 仍在运行
- 最后一次活动是什么
- 多久前

### 4.2 问题：终端宽度

**现象：** 分隔线不填满窗口

**解决方案：**
```rust
fn terminal_width() -> usize {
    Term::stdout().size().1 as usize
}
```

使用 `console::Term` 获取真实终端宽度，不依赖环境变量。

### 4.3 问题：中文退格

**现象：** 退格中文字符导致显示错乱

**解决方案：**
```rust
let width = if c.is_ascii() { 1 } else { 2 };
for _ in 0..width {
    print!("\x08 \x08");
}
```

根据字符类型计算显示宽度。

## 5. 实现清单

### 5.1 已实现 ✓

- [x] Nerd Font 图标系统
- [x] 工具执行显示（名称+参数+状态+耗时）
- [x] AI Response 对齐
- [x] 终端宽度自适应
- [x] 中文输入处理
- [x] 交互式 /resume
- [x] **P0** 阶段指示器（Exploring/Analyzing/Synthesizing/Executing）
- [x] **P0** Subagent 状态显示
- [x] **P1** 长时间任务进度条（30秒后显示）
- [x] **P1** 活动心跳指示器（每5秒更新）

### 5.2 待实现

- [ ] **P2** Token 使用实时显示
- [ ] **P2** 工具输出折叠/展开

## 6. 设计原则

### 6.1 始终保持反馈

**原则：** 用户在任何时候都应该能看出 agent 在做什么。

- 每个操作都有视觉反馈
- 长时间操作有进度指示
- 后台任务有状态显示

### 6.2 信息层次

**原则：** 重要信息突出，次要信息收敛。

1. **关键信息**：AI 响应、错误 - 完整显示
2. **进度信息**：工具调用 - 简洁显示
3. **调试信息**：详细日志 - 仅在 verbose 模式

### 6.3 一致性

**原则：** 相同类型的信息使用相同的格式。

- 所有缩进：2 空格
- 所有图标：Nerd Font（带 ASCII fallback）
- 所有时间：毫秒或 `Xm Ys` 格式
- 所有状态：✓ 成功 / ✗ 失败

### 6.4 终端兼容性

**原则：** 支持各种终端环境。

- 宽度：动态检测
- 颜色：支持无色模式
- 图标：Nerd Font + ASCII fallback
- Unicode：正确处理 CJK 字符宽度

## 7. 代码示例

### 7.1 添加新状态显示

```rust
// sage-core/src/agent/unified/step_execution.rs

// 在执行开始时显示阶段
fn show_phase(phase: &str, description: &str) {
    println!();
    println!("  {} {}  {}",
        "󰚩".bright_cyan(),
        "sage".bright_cyan().bold(),
        format!("Phase: {}", phase).bright_white()
    );
    println!("    {}", description.dimmed());
}
```

### 7.2 添加心跳指示器

```rust
// 每 5 秒更新一次状态
fn show_heartbeat(last_activity: &str, elapsed: Duration) {
    print!("\r\x1B[K");  // 清除当前行
    print!("  {} Working... (last: {} {}s ago)",
        "●".bright_green(),
        last_activity,
        elapsed.as_secs()
    );
    std::io::stdout().flush().ok();
}
```

## 8. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2025-01-08 | 0.1.0 | 初始文档，定义基础规范 |
| 2025-01-08 | 0.1.1 | 添加开放式任务问题分析 |
| 2025-01-08 | 0.2.0 | 实现 P0/P1 功能：阶段指示器、Subagent状态、进度条、心跳指示器 |
