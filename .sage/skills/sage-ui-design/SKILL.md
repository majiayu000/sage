---
name: sage-ui-design
description: Sage CLI UI 设计规范，参考 Claude Code 的终端显示模式，包含对齐、颜色、图标等设计标准
when_to_use: 当修改 UI 显示、输出格式、颜色方案、动画效果时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 90
---

# Sage UI 设计规范

参考 Claude Code 的终端 UI 设计，确保一致的用户体验。

## 核心设计原则

### 1. 统一对齐

**所有内容从同一列开始，无前导空格**

```
● 助手回复内容从这里开始
  换行后的内容与上一行文字对齐（2空格缩进）

● Read(README.md)
  └ Read 341 lines

✱ Thinking... (ctrl+c to interrupt)
```

**关键规则：**
- 图标 `●` 从第 0 列开始（无前导空格）
- 换行内容缩进 2 空格，与图标后的文字对齐
- 工具结果使用 `└` 树形符号，缩进 2 空格

### 2. 颜色方案

#### 基础颜色表

| 元素 | 颜色 | Rust 实现 | 说明 |
|------|------|-----------|------|
| 用户输入提示 `>` | 橙色/棕色 | `.truecolor(204, 120, 50)` 或 `.yellow()` | 用户输入区域标识 |
| 助手回复图标 `●` | 亮白色 | `.bright_white()` | 内容消息标识 |
| 工具图标 `●` | 亮蓝色 | `.bright_blue()` | 工具调用标识 |
| 工具名称 | 亮白色加粗 | `.bright_white().bold()` | 突出工具名 |
| 工具参数 | 暗淡灰色 | `.dimmed()` | 次要信息 |
| 工具结果符号 `└` | 暗淡灰色 | `.dimmed()` | 树形结构 |
| 工具结果内容 | 暗淡灰色 | `.dimmed()` | 次要输出 |
| Thinking 动画 | 颜色可配置 | `.bright_blue().bold()` 等 | 根据状态变化 |
| Thinking 完成 `✓` | 亮青色暗淡 | `.bright_cyan().dimmed()` | 完成状态 |
| 错误信息 | 红色 | `.red()` | 错误提示 |
| 成功信息 | 绿色 | `.green()` | 成功提示 |
| 警告信息 | 黄色 | `.yellow()` | 警告提示 |

#### 动画颜色配置

```rust
// animation.rs 中的颜色映射
let colored_output = match color {
    "blue" => format!("{} {}", frame, message).bright_blue().bold(),
    "green" => format!("{} {}", frame, message).bright_green().bold(),
    "yellow" => format!("{} {}", frame, message).bright_yellow().bold(),
    "red" => format!("{} {}", frame, message).bright_red().bold(),
    "cyan" => format!("{} {}", frame, message).bright_cyan().bold(),
    "magenta" => format!("{} {}", frame, message).bright_magenta().bold(),
    _ => format!("{} {}", frame, message).bright_white().bold(),
};
```

#### 使用 `colored` crate

```rust
use colored::Colorize;

// 基础颜色
"text".red()           // 红色
"text".green()         // 绿色
"text".yellow()        // 黄色
"text".blue()          // 蓝色
"text".cyan()          // 青色
"text".magenta()       // 品红

// 亮色变体
"text".bright_white()  // 亮白色
"text".bright_blue()   // 亮蓝色
"text".bright_cyan()   // 亮青色

// 样式修饰
"text".bold()          // 加粗
"text".dimmed()        // 暗淡
"text".italic()        // 斜体

// 组合使用
"text".bright_white().bold()      // 亮白加粗
"text".bright_cyan().dimmed()     // 亮青暗淡

// 自定义 RGB 颜色
"text".truecolor(204, 120, 50)    // 橙色/棕色
```

### 3. 图标系统

```rust
// 核心图标（sage-core/src/ui/icons.rs）
pub const MESSAGE: &str = "●";      // 助手消息
pub const RESULT: &str = "└";       // 工具结果（树形）
pub const COGITATE: &str = "✱";     // Thinking 状态
pub const CHECKMARK: &str = "✓";    // 完成状态
pub const PROMPT: &str = "❯";       // 用户输入提示
```

## 输出格式规范

### 助手消息

```
● 消息内容第一行
  消息内容第二行（2空格缩进）
  消息内容第三行
```

**实现：**
```rust
fn on_content_start(&self) {
    print!("{} ", Icons::message().bright_white());
}

fn on_content_chunk(&self, chunk: &str) {
    // 换行后添加2空格缩进
    let indented = chunk.replace('\n', "\n  ");
    print!("{}", indented);
}
```

### 工具调用

```
● Read(README.md)
  └ Read 341 lines

● Bash(git status)
  └ On branch main
    Your branch is up to date

● Task(探索代码库结构)
  └ Found 15 files...
```

**工具动画显示格式：**
```
◐ Running (3.2s) · Read
◐ Running (5.1s) · 探索代码库结构
```

注意：
- 不显示 "Step 1" 等步骤编号
- Task 工具显示实际的 description 内容，而非 "Task"
- 动画格式：`{spinner} Running ({elapsed}s) · {detail}`

**实现：**
```rust
fn on_tool_start(&self, name: &str, params: &str) {
    println!();
    print!("{} {}", Icons::message().bright_white(), name.bright_white().bold());
    if !params.is_empty() {
        println!("({})", params.dimmed());
    }
}

fn on_tool_result(&self, output: &str) {
    // 结果缩进2空格，多行内容额外缩进
    let indented = output.replace('\n', "\n    ");
    println!("  {} {}", Icons::result().dimmed(), indented.dimmed());
}
```

### Thinking 状态

**进行中：**
```
✱ Thinking... (ctrl+c to interrupt · thinking)
```

**完成后：**
```
✓ Thought for 2.1s
```

**实现：**
```rust
// 动画运行时（animation.rs）
async fn run_animation(...) {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    // 显示: "⠋ Thinking (2.3s)"
    print!("\r{} {} ({:.1}s)", frame, message, elapsed);
}

// 停止时显示完成状态
pub async fn stop_animation(&self) {
    let completion_msg = format!("✓ Thought for {:.1}s", elapsed);
    println!("{}", completion_msg.bright_cyan().dimmed());
}
```

## 关键文件

| 文件 | 职责 |
|------|------|
| `sage-core/src/ui/icons.rs` | 图标定义 |
| `sage-core/src/ui/animation.rs` | Thinking 动画 |
| `sage-core/src/output/strategy.rs` | 输出策略（Streaming/Batch/JSON） |
| `sage-cli/src/ui/nerd_console.rs` | CLI 控制台 |

## 对齐检查清单

修改 UI 时，确保：

- [ ] 所有图标从第 0 列开始（无前导空格）
- [ ] 换行内容使用 2 空格缩进
- [ ] 工具结果使用 `└` 符号 + 2 空格缩进
- [ ] 多行工具输出额外缩进 2 空格（共 4 空格）
- [ ] Thinking 完成消息与其他内容对齐
- [ ] 颜色使用符合规范

## 常见问题

### Q: 为什么换行后内容没对齐？

检查 `on_content_chunk` 是否正确处理换行：
```rust
let indented = chunk.replace('\n', "\n  ");  // 2空格
```

### Q: 图标显示宽度不一致？

Unicode 图标在不同终端可能有不同宽度。使用 `unicode-width` crate 计算实际宽度：
```rust
use unicode_width::UnicodeWidthStr;
let width = icon.width();
```

### Q: 如何测试对齐效果？

```bash
# 运行简单测试
sage "你好"

# 检查多行输出
sage "列出这个项目的主要功能"

# 检查工具调用显示
sage "读取 README.md"
```

## 参考：Claude Code UI 模式

Claude Code 使用 React/Ink 实现终端 UI：

```javascript
// 统一的 FlexBox 布局
<FlexBox flexDirection="column" width="100%">
  <Text>{content}</Text>
</FlexBox>

// 工具显示
<FlexBox marginTop={1}>
  <Text>● </Text>
  <Text bold>{toolName}</Text>
  <Text dimColor>({params})</Text>
</FlexBox>
<FlexBox paddingLeft={2}>
  <Text dimColor>└ {result}</Text>
</FlexBox>
```

Sage 使用 Rust 直接输出到终端，需要手动管理缩进和对齐。

## 流式输出架构

### 数据流路径

```
LLM API (SSE) → SseDecoder → StreamChunk → LlmOrchestrator → OutputStrategy → Terminal
```

### 关键组件

1. **SseDecoder** (`sage-core/src/llm/sse_decoder/mod.rs`)
   - 解析 SSE 事件流
   - 处理不完整的 UTF-8 序列
   - 按 `\n\n` 分割事件

2. **Provider** (`sage-core/src/llm/providers/*.rs`)
   - 将 SSE 事件转换为 `StreamChunk`
   - 处理 `content_block_delta` 事件中的 `text_delta`

3. **LlmOrchestrator** (`sage-core/src/agent/unified/llm_orchestrator.rs`)
   - 调用 `output_strategy.on_content_chunk()` 显示每个 chunk

4. **OutputStrategy** (`sage-core/src/output/strategy.rs`)
   - `StreamingOutput`: 实时显示每个 chunk
   - `BatchOutput`: 收集所有内容后一次显示

### 流式输出实现

```rust
// llm_orchestrator.rs - stream_chat_with_animation_stop
loop {
    select! {
        chunk_opt = stream.next() => {
            match chunk_opt {
                Some(Ok(chunk)) => {
                    if let Some(ref chunk_content) = chunk.content {
                        if !chunk_content.is_empty() {
                            // 停止动画（仅第一次）
                            if !animation_stopped {
                                event_manager.stop_animation().await;
                                animation_stopped = true;
                            }
                            // 开始内容显示
                            if !has_content {
                                output_strategy.on_content_start();
                                has_content = true;
                            }
                            // 实时显示 chunk
                            output_strategy.on_content_chunk(chunk_content);
                            content.push_str(chunk_content);
                        }
                    }
                }
                // ...
            }
        }
    }
}
```

### 常见流式问题

1. **代理服务器缓冲**：某些 API 代理会缓冲整个响应后再转发
2. **模型生成模式**：某些模型可能生成大块内容而非逐 token
3. **网络缓冲**：HTTP 客户端或网络层可能有缓冲

### 调试流式输出

```rust
// 在 on_content_chunk 中添加调试
fn on_content_chunk(&self, chunk: &str) {
    tracing::debug!("Received chunk: {} bytes", chunk.len());
    let indented = chunk.replace('\n', "\n  ");
    print!("{}", indented);
    let _ = io::stdout().flush();
}
```
