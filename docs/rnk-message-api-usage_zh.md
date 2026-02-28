# rnk Message API 使用指南

## 概述

Sage CLI 已经更新为使用 rnk 0.6.0 的 `Message` 组件，替代了之前的自定义渲染函数。这次重构简化了代码，提高了一致性，并减少了约 80 行代码。

## 主要变更

### 之前的实现（自定义渲染函数）

```rust
// 需要手动构建 Element 树
fn render_user_message(text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
        .child(Text::new(text).color(Color::BrightWhite).into_element())
        .into_element()
}

fn render_thinking() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Magenta).into_element())
        .child(Text::new("Thinking...").color(Color::Magenta).into_element())
        .into_element()
}

// 使用 ANSI 转义序列直接打印
println!("\x1b[35m● Thinking...\x1b[0m");
```

### 现在的实现（rnk Message 组件）

```rust
use rnk::components::Message;

// 简洁的 API
rnk::println(Message::user("Hello, world!").into_element());
rnk::println(Message::assistant("Hi! How can I help?").into_element());
rnk::println(Message::tool("Thinking...").into_element());
rnk::println(Message::error("Something went wrong").into_element());
```

## rnk Message API 详解

### 可用的消息类型

rnk 提供了以下消息类型，每种都有预定义的样式和前缀：

| 方法 | 角色 | 前缀 | 颜色 | 用途 |
|------|------|------|------|------|
| `Message::user()` | User | `> ` | Yellow | 用户输入消息 |
| `Message::assistant()` | Assistant | `● ` | BrightWhite | AI 助手响应 |
| `Message::system()` | System | `● ` | Cyan | 系统消息 |
| `Message::tool()` | Tool | `● ` | Magenta | 工具调用/思考状态 |
| `Message::tool_result()` | ToolResult | `  ⎿ ` | Gray | 工具执行结果 |
| `Message::error()` | Error | `● ` | Red | 错误消息 |

### 基本使用

```rust
use rnk::components::Message;

// 1. 用户消息
rnk::println(Message::user("Help me refactor the code").into_element());

// 2. 助手响应
rnk::println(Message::assistant("I'll help you with that.").into_element());

// 3. 工具调用
rnk::println(Message::tool("read_file").into_element());

// 4. 错误消息
rnk::println(Message::error("File not found").into_element());
```

### 自定义前缀

如果需要自定义前缀，可以使用 `.prefix()` 方法：

```rust
let msg = Message::tool("Processing...")
    .prefix("⚙️  ")
    .into_element();
rnk::println(msg);
```

## 其他 rnk 组件

除了 `Message`，rnk 还提供了其他有用的组件：

### ToolCall 组件

用于显示工具调用及其参数：

```rust
use rnk::components::ToolCall;

let tool = ToolCall::new("read_file", "path=/tmp/test.txt");
rnk::println(tool.into_element());
// 输出: ● read_file(path=/tmp/test.txt)
```

### ThinkingBlock 组件

用于显示 AI 的思考过程：

```rust
use rnk::components::ThinkingBlock;

let thinking = ThinkingBlock::new("Analyzing the problem...\nConsidering options...");
rnk::println(thinking.into_element());
```

## 核心 API

### rnk::println()

`rnk::println()` 是核心 API，支持打印文本和 Element：

```rust
// 打印文本
rnk::println("Simple text message");

// 打印 Element
rnk::println(Message::user("Hello").into_element());

// 打印格式化文本
rnk::println(format!("Downloaded {} files", count));
```

### render_to_string()

如果需要将 Element 渲染为字符串（例如用于测试或日志），可以使用：

```rust
use rnk::render_to_string;

let element = Message::user("Test").into_element();
let output = render_to_string(&element, 80); // 80 是终端宽度
println!("{}", output);
```

## 优势

### 1. 代码简化

- **之前**: 需要手动构建 Element 树，约 120 行代码
- **现在**: 使用 Message API，约 40 行代码
- **减少**: ~80 行代码（67% 减少）

### 2. 一致性

所有消息使用统一的样式和前缀，确保 UI 的一致性。

### 3. 可维护性

- 样式集中在 rnk 库中管理
- 修改样式只需更新 rnk，不需要修改应用代码
- 减少了重复代码

### 4. 类型安全

使用 Rust 的类型系统确保消息类型正确，避免了手动拼接 ANSI 转义序列的错误。

## 迁移指南

如果你有使用旧 API 的代码，可以按照以下步骤迁移：

### 步骤 1: 添加导入

```rust
use rnk::components::Message;
```

### 步骤 2: 替换自定义渲染函数

```rust
// 之前
fn render_user_message(text: &str) -> Element { ... }
rnk::println(render_user_message("Hello"));

// 之后
rnk::println(Message::user("Hello").into_element());
```

### 步骤 3: 替换 ANSI 转义序列

```rust
// 之前
println!("\x1b[35m● Thinking...\x1b[0m");

// 之后
rnk::println(Message::tool("Thinking...").into_element());
```

### 步骤 4: 移除自定义渲染函数

删除不再需要的 `render_*` 函数。

## 示例：完整的事件打印器

```rust
use rnk::components::Message;

struct EventPrinter;

impl EventPrinter {
    fn print_thinking_start() {
        rnk::println(Message::tool("Thinking...").into_element());
    }

    fn print_thinking_stop() {
        // 清除思考行
        print!("\x1b[1A\x1b[2K");
        io::stdout().flush().ok();
    }

    fn print_tool_call(name: &str) {
        rnk::println(Message::tool(name).into_element());
    }

    fn print_error(message: &str) {
        rnk::println(Message::error(message).into_element());
    }

    fn print_assistant_response(text: &str) {
        rnk::println(Message::assistant(text).into_element());
    }
}
```

## 注意事项

### 1. inline 模式 vs fullscreen 模式

`rnk::println()` 只在 inline 模式下工作。在 fullscreen 模式下，消息会被忽略。

```rust
// inline 模式（默认）
render(app).run()?;

// fullscreen 模式（println 不工作）
render(app).fullscreen().run()?;
```

### 2. 跨线程使用

`rnk::println()` 是线程安全的，可以从任何线程调用：

```rust
use std::thread;

thread::spawn(|| {
    rnk::println(Message::tool("Background task completed").into_element());
});
```

### 3. 性能考虑

`rnk::println()` 会触发重新渲染。如果需要打印大量消息，考虑批量处理或使用标准的 `println!()` 进行调试输出。

## 相关资源

- [rnk 文档](https://github.com/yourusername/rnk)
- [Message 组件源码](/Users/apple/Desktop/code/AI/tool/tink/src/components/message.rs)
- [Sage CLI 实现](/Users/apple/Desktop/code/AI/code-agent/sage/crates/sage-cli/src/app.rs)

## 版本历史

- **0.2.5** (2026-01-12): 迁移到 rnk Message API
- **0.2.4**: 使用 rnk 0.4.0 的 println() API
- **0.2.3**: 使用 glm_chat 模式的自定义渲染
