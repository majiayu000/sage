# 流式消息截断问题 - 解决方案设计

**日期**: 2026-01-24
**相关文档**: `streaming-message-truncation.md`

## 方案概览

| 方案 | 修改范围 | 复杂度 | 优点 | 缺点 | 推荐度 |
|------|----------|--------|------|------|--------|
| 方案 1 | sage-cli (background_loop) | 低 | 简单、快速 | 无实时流式显示 | ⭐⭐⭐ |
| 方案 2 | sage-core (AppState) | 中 | 架构更清晰 | 需要改变 API | ⭐⭐⭐⭐ |
| 方案 3 | sage-cli (实时更新) | 高 | 完整流式体验 | 实现复杂 | ⭐⭐⭐⭐⭐ |
| 方案 4 | tink (StaticOutput 组件) | 中 | 可复用、性能好 | 需要修改 tink | ⭐⭐⭐⭐⭐ |
| 方案 5 | tink (Message 替换机制) | 低 | 优雅、通用 | 需要扩展 tink API | ⭐⭐⭐⭐⭐ |

---

## 方案 1: 修复 background_loop 计数逻辑（Quick Fix）

### 设计思路

只打印已完成的消息，忽略流式传输中的临时消息。

### 实现方案

**文件**: `crates/sage-cli/src/ui/rnk_app/executor.rs`

```rust
// 修改前
let messages = app_state.display_messages();
let new_count = messages.len();

let new_messages: Vec<_> = if new_count > ui_state.printed_count {
    let msgs: Vec<_> = messages
        .iter()
        .skip(ui_state.printed_count)
        .map(|msg| format_message(msg))
        .collect();
    ui_state.printed_count = new_count;  // ❌ 问题
    msgs
} else {
    Vec::new()
};

// 修改后
let all_messages = app_state.display_messages();
let completed_messages = &app_state.messages;  // 只获取已完成的消息
let completed_count = completed_messages.len();

let new_messages: Vec<_> = if completed_count > ui_state.printed_count {
    let msgs: Vec<_> = completed_messages
        .iter()
        .skip(ui_state.printed_count)
        .map(|msg| format_message(msg))
        .collect();
    ui_state.printed_count = completed_count;  // ✅ 只计数已完成的
    msgs
} else {
    Vec::new()
};
```

### 优点
- ✅ 修改最小，风险低
- ✅ 立即可用
- ✅ 不影响现有架构

### 缺点
- ❌ 无法看到流式传输的实时进度
- ❌ 用户体验不如实时流式显示

### 适用场景
快速修复，临时解决方案

---

## 方案 2: 重构 AppState API（Architectural Fix）

### 设计思路

将已完成的消息和流式内容分离，提供独立的 API。

### 实现方案

**文件**: `crates/sage-core/src/ui/bridge/state.rs`

```rust
impl AppState {
    /// 获取已完成的消息（不包含流式内容）
    pub fn completed_messages(&self) -> Vec<Message> {
        self.messages.clone()
    }

    /// 获取当前流式内容（如果有）
    pub fn current_streaming(&self) -> Option<StreamingContent> {
        self.streaming_content.clone()
    }

    /// 获取所有显示消息（包含流式内容） - 向后兼容
    #[deprecated(note = "Use completed_messages() and current_streaming() instead")]
    pub fn display_messages(&self) -> Vec<Message> {
        // 保持现有实现
    }
}
```

**文件**: `crates/sage-cli/src/ui/rnk_app/executor.rs`

```rust
// 修改后
let completed_messages = app_state.completed_messages();
let streaming = app_state.current_streaming();
let completed_count = completed_messages.len();

// 打印已完成的消息
let new_messages: Vec<_> = if completed_count > ui_state.printed_count {
    // ...
};

// 可选：显示流式进度指示器
if let Some(stream) = streaming {
    // 显示 "Typing..." 或进度条
}
```

### 优点
- ✅ API 更清晰，语义明确
- ✅ 向后兼容
- ✅ 为未来扩展打下基础

### 缺点
- ❌ 需要修改 API
- ❌ 仍然没有实时流式显示

### 适用场景
中长期解决方案，适合重构

---

## 方案 3: 实现实时流式更新（Full Streaming UX）

### 设计思路

在 background_loop 中实时更新流式消息的显示，提供完整的流式体验。

### 实现方案

**文件**: `crates/sage-cli/src/ui/rnk_app/executor.rs`

```rust
// 新增：流式消息状态
struct StreamingMessageState {
    message_index: usize,  // 正在流式传输的消息索引
    last_printed_length: usize,  // 上次打印时的长度
}

let completed_messages = app_state.completed_messages();
let streaming = app_state.current_streaming();

// 1. 打印已完成的新消息
let new_completed: Vec<_> = if completed_messages.len() > ui_state.printed_count {
    completed_messages
        .iter()
        .skip(ui_state.printed_count)
        .map(|msg| format_message(msg))
        .collect()
} else {
    Vec::new()
};

for msg in new_completed {
    rnk::println(msg);
    rnk::println("");
    ui_state.printed_count += 1;
}

// 2. 实时更新流式消息
if let Some(stream) = streaming {
    if stream.buffer.len() > ui_state.streaming_state.last_printed_length {
        // 方案 3A: 使用 ANSI 清除 + 重打印
        // 清除上一行，打印新内容
        print!("\r\x1b[K");
        print!("{}", format_streaming(&stream.buffer));
        flush();

        // 方案 3B: 只打印增量（更高效但需要追踪位置）
        let new_content = &stream.buffer[ui_state.streaming_state.last_printed_length..];
        print!("{}", new_content);
        flush();

        ui_state.streaming_state.last_printed_length = stream.buffer.len();
    }
} else if ui_state.streaming_state.last_printed_length > 0 {
    // 流式完成，打印换行
    println!();
    ui_state.streaming_state.last_printed_length = 0;
}
```

### 优点
- ✅ 完整的流式体验，类似 ChatGPT
- ✅ 用户可以实时看到 AI 的思考过程
- ✅ 更好的用户体验

### 缺点
- ❌ 实现复杂，需要处理光标定位
- ❌ ANSI 控制码可能在某些终端不兼容
- ❌ 与 rnk 的 inline mode 可能冲突

### 适用场景
追求极致用户体验的场景

---

## 方案 4: 扩展 tink - StaticOutput 组件（推荐 ⭐⭐⭐⭐⭐）

### 设计思路

在 tink 中添加一个 `StaticOutput` 组件，专门用于打印静态内容（不参与布局计算），类似 Bubble Tea 的 `Println`。

### tink 架构分析

当前 tink 的 `println` 实现：
1. 接收 `Element` 或文本
2. 调用 `render_to_string(element, width)` 渲染
3. 调用 `terminal.println(rendered)` 打印

问题：每次渲染都是独立的，无法"更新"已打印的内容。

### 实现方案

#### 4.1 在 tink 中添加 `println_replace` API

**文件**: `tink/src/renderer/app.rs`

```rust
/// 打印消息，如果相同 ID 的消息已存在则替换
///
/// # 使用场景
/// - 流式更新同一条消息
/// - 进度条更新
/// - 实时日志更新
///
/// # Example
/// ```ignore
/// // 第一次打印
/// rnk::println_with_id("msg-1", "Loading...");
///
/// // 稍后替换
/// rnk::println_replace("msg-1", "Loading... 50%");
///
/// // 再次替换
/// rnk::println_replace("msg-1", "Loading... 100%");
///
/// // 完成后固定
/// rnk::println_finalize("msg-1", "Loading complete!");
/// ```
pub fn println_with_id(id: impl Into<String>, message: impl IntoPrintable) {
    if let Some(sink) = current_app_sink() {
        sink.println_with_id(id.into(), message.into_printable());
    }
}

pub fn println_replace(id: impl Into<String>, message: impl IntoPrintable) {
    if let Some(sink) = current_app_sink() {
        sink.println_replace(id.into(), message.into_printable());
    }
}

pub fn println_finalize(id: impl Into<String>, message: impl IntoPrintable) {
    if let Some(sink) = current_app_sink() {
        sink.println_finalize(id.into(), message.into_printable());
    }
}
```

#### 4.2 Terminal 实现

**文件**: `tink/src/renderer/terminal.rs`

```rust
pub struct Terminal {
    // 现有字段...

    /// 追踪已打印消息的位置（用于替换）
    printed_lines: HashMap<String, PrintedMessage>,
}

struct PrintedMessage {
    id: String,
    line_start: usize,  // 消息开始的行号（从底部计算）
    line_count: usize,  // 消息占用的行数
    is_finalized: bool, // 是否已固定（不再更新）
}

impl Terminal {
    /// 打印带 ID 的消息（可替换）
    pub fn println_with_id(&mut self, id: String, message: &str) -> io::Result<()> {
        let lines: Vec<&str> = message.lines().collect();
        let line_count = lines.len();

        // 记录位置
        self.printed_lines.insert(id.clone(), PrintedMessage {
            id,
            line_start: self.current_line,
            line_count,
            is_finalized: false,
        });

        // 打印
        self.println(message)
    }

    /// 替换已打印的消息
    pub fn println_replace(&mut self, id: &str, message: &str) -> io::Result<()> {
        if let Some(old_msg) = self.printed_lines.get(id) {
            if old_msg.is_finalized {
                return Ok(()); // 已固定，不再更新
            }

            let new_lines: Vec<&str> = message.lines().collect();
            let new_line_count = new_lines.len();

            // 计算光标需要向上移动的行数
            let lines_from_bottom = self.current_line - old_msg.line_start;

            // 移动光标到原始位置
            self.move_cursor_up(lines_from_bottom)?;

            // 清除旧内容
            for _ in 0..old_msg.line_count {
                self.clear_line()?;
                self.move_cursor_down(1)?;
            }

            // 回到起始位置
            self.move_cursor_up(old_msg.line_count)?;

            // 打印新内容
            for line in new_lines {
                self.write_line(line)?;
            }

            // 移动光标回到底部
            if new_line_count < old_msg.line_count {
                self.move_cursor_down(old_msg.line_count - new_line_count)?;
            }

            // 更新记录
            self.printed_lines.insert(id.to_string(), PrintedMessage {
                id: id.to_string(),
                line_start: old_msg.line_start,
                line_count: new_line_count,
                is_finalized: false,
            });
        } else {
            // ID 不存在，直接打印
            self.println_with_id(id.to_string(), message)?;
        }

        Ok(())
    }

    /// 固定消息（不再允许更新）
    pub fn println_finalize(&mut self, id: &str, message: &str) -> io::Result<()> {
        self.println_replace(id, message)?;

        if let Some(msg) = self.printed_lines.get_mut(id) {
            msg.is_finalized = true;
        }

        Ok(())
    }
}
```

### sage 中的使用

**文件**: `crates/sage-cli/src/ui/rnk_app/executor.rs`

```rust
// 打印已完成的消息（固定）
for (i, msg) in new_completed_messages.iter().enumerate() {
    let msg_id = format!("msg-{}", ui_state.printed_count + i);
    rnk::println_with_id(msg_id.clone(), format_message(msg));
    rnk::println_finalize(msg_id, format_message(msg));  // 立即固定
    ui_state.printed_count += 1;
}

// 实时更新流式消息
if let Some(stream) = streaming {
    let streaming_id = format!("streaming-{}", ui_state.printed_count);
    rnk::println_replace(streaming_id, format_streaming(&stream.buffer));
} else if ui_state.was_streaming {
    // 流式完成，固定最后一条消息
    let streaming_id = format!("streaming-{}", ui_state.printed_count - 1);
    rnk::println_finalize(streaming_id, "");  // 固定
    ui_state.was_streaming = false;
}
```

### 优点
- ✅ 完整的流式体验
- ✅ API 优雅，语义清晰
- ✅ 可复用到其他 tink 应用（进度条、实时日志等）
- ✅ 性能好，只更新需要的部分
- ✅ 与 rnk 的架构完美契合

### 缺点
- ❌ 需要修改 tink 库
- ❌ 需要处理终端滚动和光标定位
- ❌ 实现复杂度中等

### 适用场景
长期方案，适合追求完美用户体验

---

## 方案 5: tink 消息替换机制 - 简化版（最推荐 ⭐⭐⭐⭐⭐）

### 设计思路

方案 4 的简化版本：不追踪每条消息的位置，而是利用 inline mode 的特性，直接在当前光标位置更新。

### 核心洞察

在 inline mode 下：
- 每次 `println` 都会添加新行并向下滚动
- 但我们可以在打印前向上移动光标，清除旧内容，然后重新打印

### 实现方案

#### 5.1 在 tink 中添加简单的 `println_update` API

**文件**: `tink/src/renderer/app.rs`

```rust
/// 更新上一次打印的消息（覆盖式）
///
/// 这会清除上一次 println_update 打印的内容并替换为新内容。
/// 适用于流式更新、进度条等场景。
///
/// # Example
/// ```ignore
/// rnk::println_update("Loading...");
/// thread::sleep(Duration::from_secs(1));
/// rnk::println_update("Loading... 50%");  // 替换上一行
/// thread::sleep(Duration::from_secs(1));
/// rnk::println("Loading complete!");     // 固定，开始新行
/// ```
pub fn println_update(message: impl IntoPrintable) {
    if let Some(sink) = current_app_sink() {
        sink.println_update(message.into_printable());
    }
}
```

#### 5.2 AppRuntime 实现

**文件**: `tink/src/renderer/app.rs`

```rust
pub struct AppRuntime {
    // 现有字段...

    /// 上次 println_update 的行数（用于清除）
    last_update_lines: Arc<AtomicUsize>,
}

trait AppSink {
    // 现有方法...

    fn println_update(&self, message: Printable);
}

impl AppSink for AppRuntime {
    fn println_update(&self, message: Printable) {
        // 将消息标记为"可更新"
        match self.println_queue.lock() {
            Ok(mut queue) => queue.push((message, true)),  // true = 可更新
            Err(poisoned) => poisoned.into_inner().push((message, true)),
        }
        self.request_render();
    }
}
```

#### 5.3 Terminal 实现

**文件**: `tink/src/renderer/terminal.rs`

```rust
impl Terminal {
    /// 打印可更新的消息（会清除上一次的内容）
    pub fn println_update(&mut self, message: &str, last_lines: usize) -> io::Result<usize> {
        // 1. 如果有上次的内容，清除它
        if last_lines > 0 {
            // 向上移动 last_lines 行
            execute!(
                self.stdout,
                crossterm::cursor::MoveUp(last_lines as u16)
            )?;

            // 清除这些行
            for _ in 0..last_lines {
                execute!(
                    self.stdout,
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                    crossterm::cursor::MoveDown(1)
                )?;
            }

            // 回到起始位置
            execute!(
                self.stdout,
                crossterm::cursor::MoveUp(last_lines as u16)
            )?;
        }

        // 2. 打印新内容
        let lines: Vec<&str> = message.lines().collect();
        let line_count = lines.len();

        for line in lines {
            writeln!(self.stdout, "{}", line)?;
        }

        self.stdout.flush()?;

        // 3. 返回新内容的行数
        Ok(line_count)
    }
}
```

#### 5.4 处理消息队列

**文件**: `tink/src/renderer/app.rs`

```rust
fn handle_println_messages(&mut self, messages: &[(Printable, bool)]) -> io::Result<()> {
    let (width, _) = Terminal::size().unwrap_or((80, 24));

    for (message, is_update) in messages {
        match message {
            Printable::Text(text) => {
                if *is_update {
                    let last_lines = self.runtime.last_update_lines.load(Ordering::SeqCst);
                    let new_lines = self.terminal.println_update(text, last_lines)?;
                    self.runtime.last_update_lines.store(new_lines, Ordering::SeqCst);
                } else {
                    // 普通打印，重置更新行数
                    self.runtime.last_update_lines.store(0, Ordering::SeqCst);
                    self.terminal.println(text)?;
                }
            }
            Printable::Element(element) => {
                let rendered = self.render_element_to_string(element, width);
                if *is_update {
                    let last_lines = self.runtime.last_update_lines.load(Ordering::SeqCst);
                    let new_lines = self.terminal.println_update(&rendered, last_lines)?;
                    self.runtime.last_update_lines.store(new_lines, Ordering::SeqCst);
                } else {
                    self.runtime.last_update_lines.store(0, Ordering::SeqCst);
                    self.terminal.println(&rendered)?;
                }
            }
        }
    }

    self.terminal.repaint();
    Ok(())
}
```

### sage 中的使用

**文件**: `crates/sage-cli/src/ui/rnk_app/executor.rs`

```rust
// 打印已完成的新消息
for msg in new_completed_messages {
    rnk::println(format_message(msg));  // 普通打印
    rnk::println("");
}

// 实时更新流式消息
if let Some(stream) = streaming {
    rnk::println_update(format_message_from_buffer(&stream.buffer));
    ui_state.is_streaming = true;
} else if ui_state.is_streaming {
    // 流式完成，固定消息
    rnk::println("");  // 普通打印一个空行，固定上一条
    ui_state.is_streaming = false;
}
```

### 优点
- ✅ API 简单，只有一个新函数 `println_update`
- ✅ 实现相对简单
- ✅ 完整的流式体验
- ✅ 无需追踪消息 ID 或位置
- ✅ 性能好，只更新最后一条消息

### 缺点
- ❌ 仍需修改 tink 库
- ❌ 只能更新最后一条消息（不过对流式场景足够）

### 适用场景
**最推荐的方案**，平衡了实现复杂度和用户体验

---

## 实现优先级建议

### 短期（1-2 天）
**方案 1**: 快速修复 background_loop
- 立即解决用户看不到完整回复的问题
- 风险低，可快速部署

### 中期（1 周）
**方案 5**: 实现 tink 的 `println_update` API
- 提供完整的流式体验
- API 简单，实现难度适中
- 可复用到其他场景

### 长期（可选）
**方案 4**: 完整的消息管理系统
- 如果需要更复杂的消息更新场景（如聊天历史中间插入消息）
- 可作为 tink 的重要功能持续演进

---

## tink 修改建议总结

如果你决定修改 tink 库，我推荐：

1. **优先实现方案 5** (`println_update`)
   - API 简单：只增加一个函数
   - 实现清晰：利用 ANSI 光标控制
   - 足够强大：覆盖 90% 的流式更新场景

2. **可选实现方案 4** (`println_with_id` + `println_replace`)
   - 如果未来需要更复杂的消息管理
   - 可以基于方案 5 逐步演进

### tink API 设计建议

```rust
// 核心 API（方案 5）
pub fn println_update(message: impl IntoPrintable);

// 扩展 API（方案 4，可选）
pub fn println_with_id(id: impl Into<String>, message: impl IntoPrintable);
pub fn println_replace(id: impl Into<String>, message: impl IntoPrintable);
pub fn println_finalize(id: impl Into<String>, message: impl IntoPrintable);
```

### 文件修改清单

#### tink 库修改（方案 5）
1. `tink/src/renderer/app.rs`
   - 添加 `println_update` 函数
   - 修改 AppRuntime 添加 `last_update_lines` 字段
   - 修改 AppSink trait 添加 `println_update` 方法
   - 修改 `handle_println_messages` 处理更新消息

2. `tink/src/renderer/terminal.rs`
   - 添加 `println_update` 方法
   - 实现 ANSI 光标控制逻辑

#### sage 修改（使用方案 5）
1. `crates/sage-cli/src/ui/rnk_app/executor.rs`
   - 修改 `background_loop` 使用 `println_update`
   - 添加流式状态追踪

---

## 测试计划

### 单元测试
```rust
#[test]
fn test_println_update_single_line() {
    // 测试单行更新
}

#[test]
fn test_println_update_multiline() {
    // 测试多行更新
}

#[test]
fn test_println_update_to_println() {
    // 测试从更新模式切换到普通模式
}
```

### 集成测试
1. 运行 sage，输入问题，观察流式响应
2. 检查是否能看到实时更新
3. 检查完成后消息是否完整显示
4. 测试多轮对话
5. 测试中断流式传输

### 性能测试
- 测试快速流式更新（高频 chunk）的性能
- 测试长消息（>1000 行）的更新性能
