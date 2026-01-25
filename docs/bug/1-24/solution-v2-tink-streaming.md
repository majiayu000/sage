# 流式消息截断问题 - 基于 tink 最新架构的解决方案

**日期**: 2026-01-25
**tink 版本**: 最新 (commit 5fc081e)
**相关文档**: `streaming-message-truncation.md`, `solution-proposals.md`

## tink 最新架构分析

### 模块结构

```
tink/src/renderer/
├── app.rs              # App 主结构，run() 方法
├── builder.rs          # AppOptions 构建器
├── registry.rs         # AppSink trait, Printable, AppRuntime
├── runtime.rs          # EventLoop 事件循环
├── static_content.rs   # StaticRenderer (用于 is_static 元素)
├── render_to_string.rs # render_to_string() 函数
├── element_renderer.rs # render_element() 函数
└── mod.rs              # 模块导出
```

### 关键发现

1. **StaticRenderer** (`static_content.rs`)
   - 用于处理组件树中 `is_static = true` 的元素
   - 会将静态内容提取出来，永久写入终端历史
   - **不是**用于 `rnk::println()` 的

2. **println 机制** (`registry.rs` + `app.rs`)
   - `AppSink::println()` 将消息放入 `println_queue`
   - `App::handle_println_messages()` 处理队列，调用 `terminal.println()`
   - 每条消息独立打印，**无法更新已打印的消息**

3. **两套独立机制**
   - Static 组件：用于组件树中的静态内容
   - println：用于独立打印消息（类似 Bubble Tea 的 Println）

---

## 推荐方案：扩展 println API

### 设计目标

在 tink 中添加 `println_update` API，支持更新上一条打印的消息。

### API 设计

```rust
// === 新增 API ===

/// 打印可更新的消息
///
/// 与 println 不同，这条消息可以被后续的 println_update 更新。
/// 适用于流式输出、进度条等场景。
pub fn println_streaming(message: impl IntoPrintable);

/// 更新上一条 streaming 消息
///
/// 清除上一条 println_streaming 的内容，打印新内容。
/// 如果没有活跃的 streaming 消息，行为等同于 println_streaming。
pub fn println_streaming_update(message: impl IntoPrintable);

/// 结束 streaming 模式
///
/// 将当前 streaming 消息固定（不再可更新），后续 println 正常工作。
pub fn println_streaming_end();
```

### 使用示例

```rust
// 流式输出场景
rnk::println_streaming("Loading...");
thread::sleep(Duration::from_millis(500));
rnk::println_streaming_update("Loading... 50%");
thread::sleep(Duration::from_millis(500));
rnk::println_streaming_update("Loading... 100%");
rnk::println_streaming_end();
rnk::println("Done!");  // 新的一行
```

---

## 实现方案

### 1. 修改 `registry.rs`

#### 1.1 扩展 Printable 枚举

```rust
/// Printable content that can be sent to println
#[derive(Clone)]
pub enum Printable {
    /// Plain text message
    Text(String),
    /// Rendered element (boxed to reduce enum size)
    Element(Box<Element>),
}

/// Println message type
#[derive(Clone)]
pub enum PrintlnMessage {
    /// Normal println (permanent)
    Normal(Printable),
    /// Start streaming mode (can be updated)
    StreamingStart(Printable),
    /// Update current streaming message
    StreamingUpdate(Printable),
    /// End streaming mode (finalize current message)
    StreamingEnd,
}
```

#### 1.2 扩展 AppSink trait

```rust
pub trait AppSink: Send + Sync {
    fn request_render(&self);
    fn println(&self, message: Printable);

    // === 新增 ===
    fn println_streaming(&self, message: Printable);
    fn println_streaming_update(&self, message: Printable);
    fn println_streaming_end(&self);

    fn enter_alt_screen(&self);
    fn exit_alt_screen(&self);
    fn is_alt_screen(&self) -> bool;
}
```

#### 1.3 修改 AppRuntime

```rust
pub(crate) struct AppRuntime {
    id: AppId,
    render_flag: Arc<AtomicBool>,
    println_queue: Mutex<Vec<PrintlnMessage>>,  // 改为 PrintlnMessage
    mode_switch_request: Mutex<Option<ModeSwitch>>,
    alt_screen_state: Arc<AtomicBool>,
}

impl AppSink for AppRuntime {
    fn println(&self, message: Printable) {
        self.push_println(PrintlnMessage::Normal(message));
    }

    fn println_streaming(&self, message: Printable) {
        self.push_println(PrintlnMessage::StreamingStart(message));
    }

    fn println_streaming_update(&self, message: Printable) {
        self.push_println(PrintlnMessage::StreamingUpdate(message));
    }

    fn println_streaming_end(&self) {
        self.push_println(PrintlnMessage::StreamingEnd);
    }

    fn push_println(&self, message: PrintlnMessage) {
        match self.println_queue.lock() {
            Ok(mut queue) => queue.push(message),
            Err(poisoned) => poisoned.into_inner().push(message),
        }
        self.request_render();
    }
}
```

### 2. 修改 `app.rs`

#### 2.1 添加 streaming 状态

```rust
pub struct App<F>
where
    F: Fn() -> Element,
{
    // ... 现有字段 ...

    /// Streaming message state
    streaming_state: StreamingState,
}

struct StreamingState {
    /// Whether we're in streaming mode
    is_streaming: bool,
    /// Number of lines in the current streaming message
    current_lines: usize,
}

impl Default for StreamingState {
    fn default() -> Self {
        Self {
            is_streaming: false,
            current_lines: 0,
        }
    }
}
```

#### 2.2 修改 handle_println_messages

```rust
fn handle_println_messages(&mut self, messages: &[PrintlnMessage]) -> std::io::Result<()> {
    // Println only works in inline mode
    if self.terminal.is_alt_screen() {
        return Ok(());
    }

    let (width, _) = Terminal::size().unwrap_or((80, 24));

    for message in messages {
        match message {
            PrintlnMessage::Normal(printable) => {
                // 如果正在 streaming，先结束它
                if self.streaming_state.is_streaming {
                    self.finalize_streaming()?;
                }
                // 正常打印
                let text = self.render_printable(printable, width);
                self.terminal.println(&text)?;
            }

            PrintlnMessage::StreamingStart(printable) => {
                // 如果已经在 streaming，先结束上一个
                if self.streaming_state.is_streaming {
                    self.finalize_streaming()?;
                }
                // 开始新的 streaming
                let text = self.render_printable(printable, width);
                let lines = text.lines().count();
                self.terminal.print(&text)?;  // 不换行
                self.streaming_state.is_streaming = true;
                self.streaming_state.current_lines = lines;
            }

            PrintlnMessage::StreamingUpdate(printable) => {
                if self.streaming_state.is_streaming {
                    // 清除当前 streaming 内容
                    self.clear_streaming_lines()?;
                    // 打印新内容
                    let text = self.render_printable(printable, width);
                    let lines = text.lines().count();
                    self.terminal.print(&text)?;
                    self.streaming_state.current_lines = lines;
                } else {
                    // 不在 streaming 模式，当作 StreamingStart 处理
                    let text = self.render_printable(printable, width);
                    let lines = text.lines().count();
                    self.terminal.print(&text)?;
                    self.streaming_state.is_streaming = true;
                    self.streaming_state.current_lines = lines;
                }
            }

            PrintlnMessage::StreamingEnd => {
                if self.streaming_state.is_streaming {
                    self.finalize_streaming()?;
                }
            }
        }
    }

    self.terminal.repaint();
    Ok(())
}

fn render_printable(&self, printable: &Printable, width: u16) -> String {
    match printable {
        Printable::Text(text) => text.clone(),
        Printable::Element(element) => render_to_string(element, width),
    }
}

fn clear_streaming_lines(&mut self) -> std::io::Result<()> {
    use crossterm::{cursor, terminal, execute};
    use std::io::{stdout, Write};

    let lines = self.streaming_state.current_lines;
    if lines == 0 {
        return Ok(());
    }

    let mut stdout = stdout();

    // 移动到行首
    execute!(stdout, cursor::MoveToColumn(0))?;

    // 向上移动并清除每一行
    for _ in 0..lines {
        execute!(
            stdout,
            terminal::Clear(terminal::ClearType::CurrentLine),
            cursor::MoveUp(1)
        )?;
    }

    // 清除最后一行
    execute!(stdout, terminal::Clear(terminal::ClearType::CurrentLine))?;

    stdout.flush()?;
    Ok(())
}

fn finalize_streaming(&mut self) -> std::io::Result<()> {
    use std::io::{stdout, Write};

    // 打印换行，固定当前内容
    writeln!(stdout())?;
    stdout().flush()?;

    self.streaming_state.is_streaming = false;
    self.streaming_state.current_lines = 0;
    Ok(())
}
```

### 3. 添加公共 API

在 `tink/src/renderer/mod.rs` 或 `tink/src/lib.rs` 中添加：

```rust
/// Print a streaming message (can be updated)
pub fn println_streaming(message: impl IntoPrintable) {
    if let Some(sink) = current_app_sink() {
        sink.println_streaming(message.into_printable());
    }
}

/// Update the current streaming message
pub fn println_streaming_update(message: impl IntoPrintable) {
    if let Some(sink) = current_app_sink() {
        sink.println_streaming_update(message.into_printable());
    }
}

/// End streaming mode
pub fn println_streaming_end() {
    if let Some(sink) = current_app_sink() {
        sink.println_streaming_end();
    }
}
```

---

## sage 端修改

### 修改 `executor.rs` 中的 `background_loop`

```rust
pub fn background_loop(
    state: SharedState,
    adapter: sage_core::ui::bridge::EventAdapter,
) {
    use super::components::{format_message, render_error, render_header};

    loop {
        std::thread::sleep(std::time::Duration::from_millis(80));

        if state.read().should_quit {
            break;
        }

        let pending_work = {
            let app_state = adapter.get_state();
            let completed_messages = &app_state.messages;  // 只获取已完成的消息
            let streaming = &app_state.streaming_content;  // 获取流式内容
            let completed_count = completed_messages.len();

            let mut ui_state = state.write();

            // ... header 和 error 处理保持不变 ...

            // 收集已完成的新消息
            let new_completed: Vec<_> = if completed_count > ui_state.printed_count {
                let msgs: Vec<_> = completed_messages
                    .iter()
                    .skip(ui_state.printed_count)
                    .map(|msg| format_message(msg))
                    .collect();
                ui_state.printed_count = completed_count;
                msgs
            } else {
                Vec::new()
            };

            // 获取流式内容（如果有）
            let streaming_content = streaming.as_ref().map(|s| {
                format_streaming_message(&s.buffer)
            });

            let was_streaming = ui_state.is_streaming;
            ui_state.is_streaming = streaming.is_some();

            (header_work, error_work, new_completed, streaming_content, was_streaming)
        };

        let (header_work, error_work, new_completed, streaming_content, was_streaming) = pending_work;

        // 打印 header
        if let Some(header) = header_work {
            rnk::println(header);
            rnk::println("");
        }

        // 打印 error
        if let Some(error) = error_work {
            rnk::println(error);
            rnk::println("");
        }

        // 打印已完成的消息
        for msg_element in new_completed {
            // 如果之前在 streaming，先结束它
            if was_streaming {
                rnk::println_streaming_end();
            }
            rnk::println(msg_element);
            rnk::println("");
        }

        // 更新流式消息
        if let Some(streaming_element) = streaming_content {
            rnk::println_streaming_update(streaming_element);
        } else if was_streaming {
            // 流式结束
            rnk::println_streaming_end();
        }

        rnk::request_render();
    }
}

fn format_streaming_message(buffer: &str) -> Element {
    // 格式化流式消息，可以添加 "..." 或光标指示器
    let term_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    let mut container = rnk::prelude::Box::new()
        .flex_direction(rnk::prelude::FlexDirection::Column);

    for line in buffer.lines() {
        let wrapped = wrap_text_with_prefix("", line, term_width);
        for w in wrapped {
            container = container.child(
                rnk::prelude::Text::new(w)
                    .color(rnk::prelude::Color::White)
                    .into_element()
            );
        }
    }

    // 添加光标指示器
    container = container.child(
        rnk::prelude::Text::new("▌")
            .color(rnk::prelude::Color::BrightBlack)
            .into_element()
    );

    container.into_element()
}
```

---

## 文件修改清单

### tink 库

| 文件 | 修改内容 |
|------|----------|
| `src/renderer/registry.rs` | 添加 `PrintlnMessage` 枚举，扩展 `AppSink` trait |
| `src/renderer/app.rs` | 添加 `StreamingState`，修改 `handle_println_messages` |
| `src/renderer/mod.rs` | 导出新的公共 API |
| `src/lib.rs` | 导出 `println_streaming*` 函数 |

### sage 库

| 文件 | 修改内容 |
|------|----------|
| `crates/sage-cli/src/ui/rnk_app/executor.rs` | 修改 `background_loop` 使用新 API |
| `crates/sage-cli/src/ui/rnk_app/components.rs` | 添加 `format_streaming_message` 函数 |

---

## 测试计划

### tink 单元测试

```rust
#[test]
fn test_streaming_basic() {
    // 测试基本的 streaming 流程
}

#[test]
fn test_streaming_update_multiple() {
    // 测试多次更新
}

#[test]
fn test_streaming_to_normal() {
    // 测试从 streaming 切换到 normal println
}

#[test]
fn test_streaming_multiline() {
    // 测试多行 streaming 消息
}
```

### sage 集成测试

1. 运行 sage，输入问题
2. 观察流式响应是否实时更新
3. 检查完成后消息是否完整显示
4. 测试多轮对话
5. 测试中断流式传输

---

## 备选方案：Quick Fix（不修改 tink）

如果暂时不想修改 tink，可以先用这个简单方案：

```rust
// executor.rs - background_loop
// 只打印已完成的消息，忽略流式内容
let completed_messages = &app_state.messages;  // 不用 display_messages()
let completed_count = completed_messages.len();

let new_messages: Vec<_> = if completed_count > ui_state.printed_count {
    let msgs: Vec<_> = completed_messages
        .iter()
        .skip(ui_state.printed_count)
        .map(|msg| format_message(msg))
        .collect();
    ui_state.printed_count = completed_count;
    msgs
} else {
    Vec::new()
};
```

这个方案：
- ✅ 立即可用，不需要修改 tink
- ✅ 消息会完整显示
- ❌ 没有实时流式显示（用户需要等待完成才能看到回复）
