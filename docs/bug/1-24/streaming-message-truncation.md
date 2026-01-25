# Bug Report: 流式消息显示截断问题

**日期**: 2026-01-24
**严重程度**: High
**影响范围**: 所有使用 rnk UI 的流式响应
**状态**: Confirmed

## 问题现象

用户输入问题后，AI 的流式响应只显示第一行就被截断，无法看到完整回复。

### 实际案例

1. **输入**: "nihao"
   - **期望输出**: 完整的欢迎消息（多行）
   - **实际输出**: "Hello! I'm Sage Agent, an interactive" （只有一行，被截断）

2. **输入**: "你是什么模型"
   - **期望输出**: 完整的模型介绍（多行）
   - **实际输出**: "我是 Claude, 由 Anthropic 开发的 AI 助手。具体来说，我" （只有一行，被截断）

### 截图证据

```
> nihao
Hello! I'm Sage Agent, an interactive
> 你是什么模型
我是 Claude, 由 Anthropic 开发的 AI 助手。具体来说，我
─────────────────────────────────────────────────────────────
```

## 技术分析

### 架构概览

```
LLM Streaming Response (多个 chunks)
    ↓
RnkOutput::on_content_chunk()
    ↓ emit ContentChunk event
EventAdapter::handle_event()
    ↓ append to streaming_content.buffer
AppState::streaming_content { buffer: String }
    ↓ (background_loop 每 80ms 轮询)
AppState::display_messages()
    ↓ 返回包含临时流式消息的列表
background_loop 检测到新消息
    ↓ 打印消息 + printed_count++
流式完成 → finish_streaming()
    ↓ 将 streaming_content 移动到 messages
background_loop 再次检查
    ↓ 发现 messages.len() == printed_count (已打印)
    ↓ 不再打印完整消息 ❌
```

### 根本原因

问题在于 **`display_messages()` 的设计** 和 **`background_loop` 的计数逻辑** 之间的竞态条件：

#### 1. display_messages() 的问题设计

**位置**: `crates/sage-core/src/ui/bridge/state.rs:221-235`

```rust
pub fn display_messages(&self) -> Vec<Message> {
    let mut messages = self.messages.clone();

    // 问题：在流式传输期间添加临时消息
    if let Some(streaming) = &self.streaming_content {
        messages.push(Message {
            role: Role::Assistant,
            content: MessageContent::Text(streaming.buffer.clone()),
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        });
    }

    messages
}
```

**问题点**:
- 流式传输期间，`streaming_content.buffer` 可能只有部分内容
- `display_messages()` 返回的消息数会动态变化：
  - 流式传输前：`messages.len()` = N
  - 流式传输中：`messages.len()` = N + 1 (包含不完整的临时消息)
  - 流式完成后：`messages.len()` = N + 1 (完整消息被添加到 `self.messages`)

#### 2. background_loop 的计数逻辑问题

**位置**: `crates/sage-cli/src/ui/rnk_app/executor.rs:348-358`

```rust
let new_messages: Vec<_> = if new_count > ui_state.printed_count {
    let msgs: Vec<_> = messages
        .iter()
        .skip(ui_state.printed_count)
        .map(|msg| format_message(msg))
        .collect();
    ui_state.printed_count = new_count;  // ❌ 关键问题
    msgs
} else {
    Vec::new()
};
```

**问题点**:
- `new_count` 来自 `display_messages().len()`，包含临时流式消息
- 当流式传输中途被轮询时：
  - `new_count` = N + 1 (包含不完整的临时消息)
  - `printed_count` = N
  - 条件满足，打印不完整的消息
  - `printed_count` 更新为 N + 1
- 当流式完成后：
  - `new_count` = N + 1 (完整消息已添加)
  - `printed_count` = N + 1
  - 条件不满足，**完整消息不会被打印** ❌

### 时序图

```
Time   | AppState                    | background_loop          | 用户看到的内容
-------|----------------------------|--------------------------|------------------
T0     | messages: []               | printed_count: 0         |
       | streaming_content: None    |                          |
-------|----------------------------|--------------------------|------------------
T1     | ContentStreamStarted       |                          |
       | streaming_content: {       |                          |
       |   buffer: ""               |                          |
       | }                          |                          |
-------|----------------------------|--------------------------|------------------
T2     | ContentChunk: "Hello! "    |                          |
       | buffer: "Hello! "          |                          |
-------|----------------------------|--------------------------|------------------
T3     | ContentChunk: "I'm Sage"   |                          |
       | buffer: "Hello! I'm Sage"  |                          |
-------|----------------------------|--------------------------|------------------
T4     |                            | 轮询检查                  |
       |                            | display_messages() 返回:  |
       |                            | [临时消息: "Hello! I'm Sage"] |
       |                            | new_count = 1            |
       |                            | printed_count = 0        |
       |                            | 打印不完整消息 ❌          | "Hello! I'm Sage"
       |                            | printed_count = 1        |
-------|----------------------------|--------------------------|------------------
T5     | ContentChunk: " Agent..."  |                          |
       | buffer: "Hello! I'm Sage   |                          |
       |   Agent, an interactive... |                          |
-------|----------------------------|--------------------------|------------------
T6     | ContentStreamEnded         |                          |
       | finish_streaming()         |                          |
       | messages: [                |                          |
       |   完整消息: "Hello! ..."   |                          |
       | ]                          |                          |
       | streaming_content: None    |                          |
-------|----------------------------|--------------------------|------------------
T7     |                            | 轮询检查                  |
       |                            | display_messages() 返回:  |
       |                            | [完整消息]               |
       |                            | new_count = 1            |
       |                            | printed_count = 1        |
       |                            | 不打印 ❌                 | (仍然只显示不完整的)
```

### 验证

通过运行 tink 的 `test_column` 示例，确认 rnk 的 Column 布局是正常工作的：

```bash
$ cd /Users/apple/Desktop/code/AI/tool/tink
$ cargo run --example test_column

Rendered output (4 lines):
---
Available slash commands:
## Built-in Commands
- /status - Show agent status
- /init - Initialize Sage in project
---
```

这说明问题**不在 rnk 框架**，而在 **sage 的消息管理逻辑**。

## 影响范围

- **所有流式响应**: 任何 AI 回复都会被截断
- **用户体验**: 严重影响可用性，用户无法看到完整回复
- **数据完整性**: 消息内容实际已完整接收并存储在 `AppState.messages` 中，只是显示逻辑有问题

## 相关文件

| 文件 | 行号 | 问题 |
|------|------|------|
| `crates/sage-core/src/ui/bridge/state.rs` | 221-235 | `display_messages()` 返回临时流式消息 |
| `crates/sage-cli/src/ui/rnk_app/executor.rs` | 348-358 | `background_loop` 计数逻辑导致重复打印被跳过 |
| `crates/sage-core/src/ui/bridge/adapter.rs` | 93-98 | 流式 chunk 处理和完成逻辑 |

## 修复方案对比

见 `solution-proposals.md`
