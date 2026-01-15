# Sage CLI UI Design Specification

> 本文档定义了 Sage CLI 的 UI 设计规范，基于 Claude Code 的设计模式。
> 所有 UI 相关的改动都必须遵循此规范。

**Version:** 1.0.0
**Last Updated:** 2025-01-15
**Status:** Active

---

## 1. 设计原则

1. **固定底部布局** - 输入框和状态栏始终可见
2. **虚拟滚动** - 高效渲染长对话，只渲染可见内容
3. **响应式状态** - UI 更新由状态变化驱动
4. **跨线程安全** - 后台执行器 + 前台 UI
5. **一致的图标系统** - 与 Claude Code 兼容的符号
6. **键盘优先** - 所有操作都可通过快捷键访问
7. **优雅降级** - 受限终端使用 ASCII 回退

---

## 2. 整体布局结构

### 2.1 屏幕布局

```
┌────────────────────────────────────────────────────────────────────┐
│ HEADER (6 行固定)                                                   │
│   第 1 行: Logo + 版本                                              │
│   第 2 行: Model/Provider 信息                                      │
│   第 3 行: 工作目录                                                  │
│   第 4 行: 空行                                                      │
│   第 5 行: 提示文本                                                  │
│   第 6 行: 空行                                                      │
├────────────────────────────────────────────────────────────────────┤
│ CONTENT AREA (flex-grow: 1, 可滚动)                                 │
│   - 消息历史                                                        │
│   - Thinking 块                                                     │
│   - 工具调用和结果                                                   │
│   - 流式输出内容                                                     │
├────────────────────────────────────────────────────────────────────┤
│ SEPARATOR (1 行)                                                    │
│   ────────────────────────────────────────                          │
├────────────────────────────────────────────────────────────────────┤
│ INPUT LINE (1 行)                                                   │
│   ❯ 用户输入文本                                                    │
├────────────────────────────────────────────────────────────────────┤
│ STATUS BAR (1 行)                                                   │
│   ⏵⏵ permissions required (shift+tab to cycle) | mouse on [100%]   │
└────────────────────────────────────────────────────────────────────┘
```

### 2.2 高度分配

| 区域 | 高度 | 说明 |
|------|------|------|
| Header | 6 行固定 | Logo、model、cwd、hints |
| Content | 动态 (flex-grow: 1) | 可滚动消息区域 |
| Separator | 1 行固定 | 水平分隔线 |
| Input | 1 行 (可扩展) | 用户输入区域 |
| Status Bar | 1 行固定 | 权限/滚动信息 |
| **底部总计** | **3 行固定** | Separator + Input + Status |

### 2.3 Flexbox 实现

```rust
RnkBox::new()
    .flex_direction(FlexDirection::Column)
    .height(Dimension::Percent(100.0))  // 全终端高度
    .child(header)                       // 固定高度
    .child(
        RnkBox::new()
            .flex_grow(1.0)              // 占用剩余空间
            .overflow_y(Overflow::Hidden)
            .child(content)
    )
    .child(separator)                    // 固定高度
    .child(input_line)                   // 固定高度
    .child(status_bar)                   // 固定高度
```

---

## 3. 滚动机制

### 3.1 虚拟滚动实现

**策略：**
- 预先计算所有消息行（包含换行处理）
- 追踪 scroll offset 作为行索引
- 只渲染 viewport 内可见的行

```rust
// 1. 构建所有行
let all_lines = build_render_lines(&messages, max_width);
let total_lines = all_lines.len();

// 2. 计算可见范围
let visible_start = scroll_offset.min(total_lines.saturating_sub(viewport_height));
let visible_end = (visible_start + viewport_height).min(total_lines);

// 3. 只渲染可见切片
render_visible_lines(&all_lines[visible_start..visible_end])
```

### 3.2 滚动控制

| 操作 | 按键 | 行为 |
|------|------|------|
| 向上滚动 1 行 | Arrow Up | offset -= 1 |
| 向下滚动 1 行 | Arrow Down | offset += 1 |
| 向上翻页 | PageUp | offset -= viewport_height |
| 向下翻页 | PageDown | offset += viewport_height |
| 滚动到顶部 | Home | offset = 0 |
| 滚动到底部 | End | offset = max |
| 鼠标滚轮 | Mouse wheel | 每次 3 行 |

### 3.3 滚动指示器

**位置：** 状态栏右侧
**格式：** `[XXX%]`

```rust
let scroll_percent = if max_scroll > 0 {
    Some(((scroll_offset as f32 / max_scroll as f32) * 100.0) as u8)
} else {
    None  // 不可滚动时不显示
};
```

### 3.4 自动滚动行为

- **自动滚动开启：** 新内容到达时且用户在底部（3 行范围内）
- **自动滚动关闭：** 用户手动向上滚动
- **恢复：** 下次用户输入或显式滚动到底部

---

## 4. 渲染模式

### 4.1 Fullscreen 模式（主要模式）

```rust
rnk::render(app).fullscreen().run()
```

**行为：**
- 使用备用屏幕缓冲区 (`\x1b[?1049h`)
- 完全控制终端
- 干净退出（恢复原始屏幕）
- 渲染期间隐藏光标

### 4.2 Inline 模式（备选）

```rust
rnk::render(app).run()  // 默认 inline
```

**行为：**
- 在当前光标位置渲染
- 内容保留在终端历史中
- 使用清除并重写策略更新

### 4.3 退出行为

| 模式 | 退出时 |
|------|--------|
| Fullscreen | 离开备用屏幕，恢复光标，显示 "Goodbye!" |
| Inline | 清除渲染内容，显示最终消息 |

---

## 5. 输入框设计

### 5.1 组件结构

```
❯ 用户输入文本█
```

| 部分 | 符号 | 样式 |
|------|------|------|
| 提示符 | `❯` | Yellow + Bold |
| 空格 | ` ` | - |
| 输入文本 | (用户输入) | White |
| 光标 | `█` | BrightWhite |

### 5.2 占位符

输入为空且空闲时：
```
❯ Try "edit base.rs to..."
```
- 占位符文本使用 dim/gray 颜色
- 第一次按键时消失

### 5.3 多行输入（未来增强）

- Shift+Enter 换行
- 输入框高度增长（最大 10 行）
- 超过最大高度时在输入框内滚动

### 5.4 输入历史

| 按键 | 操作 |
|------|------|
| Arrow Up | 上一条历史记录 |
| Arrow Down | 下一条历史记录 |
| 存储数量 | 最近 100 条 |
| 持久化 | 可选保存到 `~/.sage/history` |

---

## 6. 状态栏设计

### 6.1 布局

```
⏵⏵ permissions required (shift+tab to cycle) | mouse on [100%]
```

| 组件 | 位置 | 内容 |
|------|------|------|
| 模式指示器 | 左侧 | `⏵⏵` |
| 模式文本 | 左侧 | 权限模式文本 |
| 切换提示 | 中间 | `(shift+tab to cycle)` |
| 鼠标状态 | 右侧 | `mouse on/off` |
| 滚动指示器 | 最右侧 | `[100%]` |

### 6.2 权限模式

| 模式 | 显示文本 | 指示器颜色 | 行为 |
|------|---------|-----------|------|
| Normal | `permissions required` | Gray | 每个危险操作需要确认 |
| Bypass | `bypass permissions on` | Yellow | 跳过确认 |
| Plan | `plan mode` | Blue | 只规划，不执行 |

**切换：** Shift+Tab 循环: Normal -> Bypass -> Plan -> Normal

### 6.3 执行期间的状态行

活动执行期间替换正常状态：

```
⠋ Thinking...
⠙ Streaming (2.3s)
⠹ Running bash (1.5s)
```

- Spinner 动画间隔 80ms
- 动态点动画间隔 400ms
- 括号内显示已用时间

---

## 7. 消息显示

### 7.1 用户消息

```
user: 用户输入的消息内容
      续行缩进对齐
```

| 元素 | 样式 |
|------|------|
| 前缀 `user:` | Bold, 默认颜色 |
| 内容 | 默认颜色 |
| 续行 | 缩进对齐到内容起始位置 |

### 7.2 Assistant 消息

```
assistant: AI 回复的内容
           续行缩进对齐
```

| 元素 | 样式 |
|------|------|
| 前缀 `assistant:` | Bold, 默认颜色 |
| 内容 | 默认颜色 |
| 续行 | 缩进对齐 |

### 7.3 Thinking 块

**折叠状态（默认）：**
```
∴ Thinking (ctrl+o to expand)
```

**展开状态：**
```
∴ Thinking...
  详细的思考内容
  跨多行显示...
```

| 元素 | 样式 |
|------|------|
| 符号 `∴` | Dim + Italic |
| 标签 | Dim + Italic |
| 内容 | Dim, 缩进 2 空格 |

**隐藏的 Thinking：**
```
✻ Thinking...
```
- 符号 `✻` 用于隐藏/保密的思考内容

### 7.4 工具调用显示

**执行中：**
```
tool: Read
  args: path="src/main.rs"
  ⠋ Running...
```

**完成（成功）：**
```
tool: Read
  args: path="src/main.rs"
  result: Read 150 lines
```

**完成（错误）：**
```
tool: Read
  args: path="nonexistent.rs"
  error: File not found
```

| 元素 | 样式 |
|------|------|
| `tool:` 前缀 | Magenta + Bold |
| 工具名称 | Magenta + Bold |
| `args:` | Magenta, 缩进 |
| 参数 | Dim |
| `result:` | Magenta, 缩进 |
| 结果内容 | 默认 |
| `error:` | Magenta, 缩进 |
| 错误内容 | Red |

### 7.5 流式输出

- 实时追加到临时消息
- 状态区域显示 spinner 指示器
- 完成后变为常规 assistant 消息

---

## 8. 动画和指示器

### 8.1 Spinner 动画

**帧（Braille 图案）：**
```rust
["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
```

**时序：** 每帧 80ms (12.5 FPS)

**用途：**
- Thinking 状态
- Streaming 状态
- 工具执行

### 8.2 进度点

**模式：** `. -> .. -> ... -> (空)`
**时序：** 每步 400ms

```rust
let dot_count = ((now_ms / 400) % 4) as usize;
let dots = ".".repeat(dot_count);
```

### 8.3 时间显示

格式：`(X.Xs)` 或 `Baked for Xs`

```rust
let elapsed = started_at.elapsed().as_secs_f32();
format!("({:.1}s)", elapsed)
```

---

## 9. 颜色方案

### 9.1 暗色主题（默认/Claude Code 风格）

| 元素 | 颜色 | ANSI 代码 |
|------|------|----------|
| 用户提示符 `❯` | Yellow + Bold | 33;1 |
| 用户文本 | White | 37 |
| Assistant 前缀 | BrightWhite | 97 |
| Assistant 文本 | White | 37 |
| Thinking | Dim (Gray) | 2 |
| 工具名称 | Magenta + Bold | 35;1 |
| 工具参数 | Dim | 2 |
| 工具结果 | Gray (Ansi256 245) | 38;5;245 |
| 错误 | Red | 31 |
| 成功 | Green | 32 |
| 警告 | Yellow | 33 |
| 信息 | Blue | 34 |
| 边框 | Dark Gray (Ansi256 238) | 38;5;238 |
| 状态栏文本 | Gray (Ansi256 245) | 38;5;245 |
| 品牌色 | Cyan | 36 |
| Git 分支 | Green | 32 |

### 9.2 亮色主题

| 元素 | 颜色变化 |
|------|---------|
| 用户提示符 | Blue (替代 Yellow) |
| Assistant | Black (替代 White) |
| 其他 | 针对亮色背景调整 |

### 9.3 主题检测

- 检查 `COLORFGBG` 环境变量
- 如果可用，检查终端主题
- 默认使用暗色主题

---

## 10. 图标系统

### 10.1 核心图标（Claude Code 风格）

| 图标 | 符号 | 用途 |
|------|------|------|
| 消息 | `●` | Assistant 消息前缀 |
| 结果 | `⎿` | 工具结果前缀 |
| 思考 | `∴` | Thinking 状态（可展开） |
| 隐藏 | `✻` | 隐藏的 thinking |
| 提示 | `❯` | 用户输入提示符 |
| 模式 | `⏵⏵` | 权限模式指示器 |

### 10.2 状态图标

| 图标 | Unicode | Nerd Font | 回退 |
|------|---------|-----------|------|
| 成功 | `✓` | `` | `✓` |
| 错误 | `✗` | `` | `✗` |
| 警告 | `⚠` | `` | `⚠` |
| 信息 | `ℹ` | `` | `ℹ` |
| 运行中 | `▶` | `` | `▶` |

### 10.3 工具图标

| 工具 | 图标 |
|------|------|
| bash | `>` / `` |
| edit/write | `<>` / `` |
| read | 文件图标 |
| grep/glob | 搜索图标 |

---

## 11. 键盘快捷键

### 11.1 全局快捷键

| 按键 | 操作 |
|------|------|
| Ctrl+C | 退出 / 取消操作 |
| Ctrl+O | 切换 thinking 展开/折叠 |
| Shift+Tab | 切换权限模式 |
| Ctrl+Y | 切换鼠标捕获（用于选择） |
| Escape | 取消当前操作 |

### 11.2 导航

| 按键 | 操作 |
|------|------|
| Arrow Up | 向上滚动 / 上一条历史 |
| Arrow Down | 向下滚动 / 下一条历史 |
| PageUp | 向上翻页 |
| PageDown | 向下翻页 |
| Home | 滚动到顶部 |
| End | 滚动到底部 |

### 11.3 输入

| 按键 | 操作 |
|------|------|
| Enter | 提交消息 |
| Backspace | 删除字符 |
| Ctrl+U | 清除输入行 |
| Ctrl+W | 删除单词 |

---

## 12. rnk 框架要求

### 12.1 已支持的功能

- Taffy Flexbox 布局
- use_signal 响应式状态
- use_input 键盘处理
- use_scroll 滚动管理
- use_mouse 鼠标事件
- Fullscreen 和 inline 模式
- Spinner 组件
- Static 输出组件
- 跨线程渲染请求

### 12.2 需要增强的功能

| 功能 | 当前状态 | 建议 |
|------|---------|------|
| 焦点管理 | 未内置 | 添加 use_focus/use_focus_manager hooks |
| TextInput 组件 | 未内置 | 添加带光标的内置文本输入 |
| 对话框系统 | 未内置 | 添加模态对话框栈 |
| 终端检测 | 未内置 | 添加能力检测 |
| 快捷键注册 | 手动 | 添加快捷键注册系统 |

---

## 13. 架构集成

### 13.1 线程模型

```
┌──────────────────┐
│   Main Thread    │
│   (rnk render)   │
└────────┬─────────┘
         │
         │ request_render()
         │
┌────────▼─────────┐
│ Background Thread│
│ (Tokio Runtime)  │
│   - Executor     │
│   - Event Loop   │
└────────┬─────────┘
         │
         │ emit_event()
         │
┌────────▼─────────┐
│  EventAdapter    │
│ (watch::channel) │
│   - AppState     │
└──────────────────┘
```

### 13.2 状态流转

1. 用户输入由 `use_input` 捕获
2. 命令通过 `mpsc::channel` 发送到执行器线程
3. 执行器处理，发出 `AgentEvent`
4. `EventAdapter` 更新 `AppState`
5. Watch channel 通知 UI 线程
6. `request_render()` 触发重新渲染
7. `app()` 函数读取状态并渲染

---

## 14. 文件引用

| 用途 | 路径 |
|------|------|
| 主 rnk app | `/crates/sage-cli/src/ui/rnk_app.rs` |
| UI 状态模型 | `/crates/sage-core/src/ui/bridge/state.rs` |
| 颜色定义 | `/crates/sage-core/src/ui/theme/colors.rs` |
| 图标系统 | `/crates/sage-core/src/ui/theme/icons.rs` |
| Spinner 组件 | `/crates/sage-core/src/ui/components/spinner.rs` |
| 状态栏 | `/crates/sage-core/src/ui/components/status_bar.rs` |
| 输入框 | `/crates/sage-core/src/ui/components/input_box.rs` |
| 事件适配器 | `/crates/sage-core/src/ui/bridge/adapter.rs` |

---

## 15. 关键指标

| 指标 | 目标值 |
|------|--------|
| 渲染 FPS | 60 (节流) |
| Spinner FPS | 12.5 (80ms 帧) |
| 输入延迟 | < 10ms |
| 滚动平滑度 | 逐行或逐页 |
| 内存 | 虚拟滚动，只渲染可见内容 |

---

## 变更日志

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2025-01-15 | 初始版本，基于 Claude Code 设计分析 |
