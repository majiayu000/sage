# RNK 库问题分析报告

## 关键发现

### 1. [高优先级] Column 布局高度计算错误

**文件:** `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/app.rs:1409`

```rust
// 当前代码 - BUG
height = height.max(child_height_sum);

// 应该是
height = height.saturating_add(child_height_sum);
```

**问题:** 对于 Column 布局，子元素高度应该累加，而不是取最大值。这导致多行内容被压缩到一行显示。

**这就是 `/commands` 输出显示异常的根本原因！**

---

### 2. [高优先级] `println()` 不更新 `previous_lines`

**文件:** `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/terminal.rs:353-372`

```rust
pub fn println(&mut self, message: &str) -> std::io::Result<()> {
    // ...
    self.clear_inline_content()?;  // 清空 previous_lines

    for line in message.lines() {
        write!(stdout, "{}{}\r\n", line, ansi::erase_end_of_line())?;
    }
    // BUG: 没有更新 previous_lines
    // 下次渲染会认为是"首次渲染"
}
```

**影响:** 光标定位可能出错，导致 UI 闪烁或错位。

---

### 3. [高优先级] Mutex 中毒被静默忽略

**文件:** `app.rs` 多处

```rust
fn take_println_messages(&self) -> Vec<Printable> {
    if let Ok(mut queue) = self.println_queue.lock() {
        std::mem::take(&mut *queue)
    } else {
        Vec::new()  // 静默返回空，不报错
    }
}
```

**影响:** 如果发生 panic，后续操作会静默失败。

---

### 4. [中优先级] 文本测量不考虑换行

**文件:** `/Users/apple/Desktop/code/AI/tool/tink/src/layout/engine.rs:160`

```rust
let text_height = text.lines().count().max(1) as f32;
// 只计算 \n 分隔的行数，不考虑文本换行
```

**影响:** 长文本换行时高度被低估。

---

### 5. [中优先级] 行尾符不一致

| 位置 | 行尾符 |
|------|--------|
| `render_to_string()` | `\n` (LF) |
| `Output::render()` | `\r\n` (CRLF) |

**影响:** 不同渲染路径输出不一致。

---

### 6. [中优先级] TOCTOU 竞态条件

**文件:** `app.rs:219-226`

```rust
fn unregister_app(id: AppId) {
    if let Ok(mut registry) = registry().lock() {
        registry.remove(&id);
    }
    // 竞态窗口：另一个线程可能在这里修改 CURRENT_APP
    if AppId::from_raw(CURRENT_APP.load(Ordering::SeqCst)) == Some(id) {
        set_current_app(None);
    }
}
```

---

### 7. [低优先级] Element Clone 在热路径创建新 ID

**文件:** `app.rs:897` - `filter_static_elements()`

每次 clone Element 都会生成新的 ID，在频繁调用的路径上造成不必要的开销。

---

### 8. [低优先级] ClipRegion 无验证

**文件:** `output.rs:77-81`

如果 `x2 <= x1` 或 `y2 <= y1`，`contains()` 永远返回 false，内容会被完全隐藏。

---

## 修复建议

### 修复 #1 (最关键)

```rust
// app.rs:1402-1410
if !element.children.is_empty() {
    let mut child_height_sum = 0u16;
    for child in &element.children {
        let child_height = self.calculate_element_height(child, max_width, _engine);
        child_height_sum = child_height_sum.saturating_add(child_height);
    }
    // 修复：Column 布局应该累加高度
    if element.style.flex_direction == crate::core::FlexDirection::Column {
        height = height.saturating_add(child_height_sum);
    } else {
        height = height.max(child_height_sum);
    }
}
```

### 修复 #2

```rust
// terminal.rs:353-372
pub fn println(&mut self, message: &str) -> std::io::Result<()> {
    // ...
    self.clear_inline_content()?;

    for line in message.lines() {
        write!(stdout, "{}{}\r\n", line, ansi::erase_end_of_line())?;
    }

    // 修复：标记需要重新渲染
    self.repaint();

    stdout.flush()?;
    Ok(())
}
```

### 修复 #3

```rust
// 使用 expect 或 unwrap_or_else 记录错误
fn take_println_messages(&self) -> Vec<Printable> {
    match self.println_queue.lock() {
        Ok(mut queue) => std::mem::take(&mut *queue),
        Err(e) => {
            tracing::error!("println_queue mutex poisoned: {}", e);
            Vec::new()
        }
    }
}
```

---

## 问题优先级总结

| 优先级 | 问题 | 影响 |
|--------|------|------|
| **P0** | Column 高度计算错误 | 多行内容显示异常 |
| **P1** | println 不更新状态 | 光标定位错误 |
| **P1** | Mutex 中毒静默忽略 | 静默失败 |
| **P2** | 文本测量不考虑换行 | 高度低估 |
| **P2** | 行尾符不一致 | 渲染差异 |
| **P2** | TOCTOU 竞态 | 潜在竞态 |
| **P3** | Clone 创建新 ID | 性能开销 |
| **P3** | ClipRegion 无验证 | 内容隐藏 |
