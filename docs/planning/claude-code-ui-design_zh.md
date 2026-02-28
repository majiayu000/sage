# Claude Code UI 设计参考

基于 open-claude-code 逆向分析，总结 Claude Code 的终端 UI 设计规范。

## 技术架构

| 项目 | 技术栈 | 渲染模式 |
|------|--------|----------|
| Claude Code | Ink (React for CLI) | 声明式组件 + 行级差分 |
| Sage | rnk (Rust) | 声明式组件 + 行级差分 |

## UI 符号规范

| 符号 | 用途 | 样式 |
|------|------|------|
| `∴` | Thinking 状态（可展开） | dim + italic |
| `✻` | Redacted Thinking | dim + italic |
| `●` | 消息/工具调用指示符 | bold |
| `⎿` | 工具结果/子内容 | dim |
| `>` | 用户输入提示符 | yellow + bold |

## 组件设计

### 1. 用户输入 (User Message)

```
> 用户输入的内容
```

样式：
- `>` 前缀：黄色 + 粗体
- 内容：白色

### 2. AI 响应 (Assistant Message)

```
● AI 的回复内容，可能是多行的
  第二行会缩进对齐
```

样式：
- `●` 前缀：白色/亮白
- 内容：普通白色
- 换行后缩进 2 空格对齐

### 3. Thinking 状态

**折叠模式（默认）**：
```
∴ Thinking (ctrl+o to expand)
```

**展开模式**：
```
∴ Thinking…
  思考的具体内容
  可能有多行...
```

**Redacted Thinking**：
```
✻ Thinking…
```

样式：
- 符号和文字：dim + italic
- 展开内容：左侧 padding 2 空格

### 4. 工具调用 (Tool Use)

**执行中**：
```
● Read("src/main.rs")
  ⎿ 正在读取...
```

**执行完成（成功）**：
```
● Read("src/main.rs")
  ⎿ Read 150 lines
```

**执行完成（失败）**：
```
● Read("src/main.rs")
  ⎿ Error: File not found
```

样式：
- `●` 前缀：蓝色/品红色
- 工具名：粗体
- 参数：dim
- `⎿` 结果前缀：dim
- 错误信息：红色

### 5. 输入框 (Input Component)

```
> _
```

特性：
- 固定在底部
- 支持多行输入
- 光标显示
- Vim 模式支持（可选）
- 历史记录（上下键）

## 布局结构

```
┌─────────────────────────────────────────┐
│ [Static 区域 - 历史消息，不会被重绘]      │
│                                         │
│ > 用户之前的输入                          │
│ ● AI 之前的回复                          │
│ ● Read("file.rs")                       │
│   ⎿ Read 100 lines                      │
│                                         │
├─────────────────────────────────────────┤
│ [动态区域 - 当前活动，会被差分更新]        │
│                                         │
│ ∴ Thinking...                           │
│ ● 正在生成的回复...                       │
│                                         │
├─────────────────────────────────────────┤
│ [输入区域 - 固定在底部]                   │
│                                         │
│ > 用户正在输入的内容_                     │
│                                         │
│ [状态栏]                                 │
│ ▶▶ bypass permissions on                │
└─────────────────────────────────────────┘
```

## 状态栏设计

位于屏幕最底部，显示：
- 权限状态：`▶▶ bypass permissions on`
- 会话信息
- 快捷键提示

## 交互行为

### 快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Ctrl+C` | 中断/退出 |
| `Ctrl+O` | 展开/折叠 Thinking |
| `Esc` | 取消当前输入 |
| `↑` / `↓` | 历史记录导航 |

### 动画效果

1. **Spinner** - 等待 AI 响应时
2. **打字机效果** - 流式输出时（可选）
3. **进度条** - 工具执行时（可选）

## 颜色方案

```rust
// 基于 ANSI 256 色
const COLORS = {
    // 前景色
    user_prompt: Yellow + Bold,
    assistant_prefix: BrightWhite,
    assistant_text: White,
    thinking: Dim + Italic,
    tool_name: Blue + Bold,
    tool_args: Dim,
    tool_result: Ansi256(245),  // 灰色
    error: Red,

    // 状态栏
    status_bar_bg: Ansi256(236),
    status_bar_fg: White,
};
```

## 与 rnk 的映射

| Claude Code (Ink) | rnk 对应 |
|-------------------|----------|
| `<Box>` | `rnk::Box` |
| `<Text>` | `rnk::Text` |
| `<Static>` | `rnk::Static` (永久输出) |
| `dimColor` | `.dim()` |
| `bold` | `.bold()` |
| `italic` | `.italic()` |
| `flexDirection="column"` | `.flex_direction(Column)` |
| `marginTop={1}` | `.margin_top(1)` |
| `paddingLeft={2}` | `.padding_left(2)` |

## 固定底部布局（研究）

基于 open-claude-code 分析，Claude Code 的固定底部布局使用 Ink 的 FlexBox：

### 技术对比

| 特性 | Claude Code (Ink) | Sage (rnk) |
|------|-------------------|------------|
| 框架 | Node.js + React | Native Rust |
| 渲染模式 | 全屏/Inline 双模式 | App/println 双模式 |
| 布局系统 | FlexBox (Yoga) | FlexBox (taffy) |
| 固定底部 | `flexGrow: 1` 撑满上方 | `min_height` 固定内容区 |

### 状态栏结构

```
┌────────────────────────────────────────────────────────┐
│ [滚动内容区 - flexGrow: 1]                              │
│                                                        │
│ ❯ 用户输入                                              │
│ ● AI 回复...                                           │
│ ● Read("file.rs")                                     │
│   ⎿ Read 100 lines                                    │
│                                                        │
├────────────────────────────────────────────────────────┤ ← 分隔线 ─
│ ❯ 用户正在输入█                                         │ ← 输入行
│ ▸▸ bypass permissions on (shift+tab to cycle)         │ ← 状态栏
└────────────────────────────────────────────────────────┘
```

### 权限模式

Claude Code 有三种权限模式，通过 Shift+Tab 循环：

| 模式 | 显示文本 | 行为 |
|------|----------|------|
| Normal | `permissions required` | 每个危险操作需要确认 |
| Bypass | `bypass permissions on` | 跳过权限确认 |
| Plan | `plan mode` | 只规划不执行 |

### Demo 实现

已创建 `examples/fixed_bottom_demo.rs`，验证 rnk 可以实现固定底部布局：

```bash
cargo run --example fixed_bottom_demo
```

特性：
- ✅ 固定底部输入框 + 状态栏
- ✅ 滚动内容区
- ✅ Shift+Tab 切换权限模式
- ✅ 分隔线
- ✅ 光标显示

## 实现优先级

### Phase 1: 基础组件
- [x] 用户消息渲染
- [x] AI 响应渲染（流式）
- [x] 工具调用显示
- [x] 工具结果显示

### Phase 2: Thinking 状态
- [x] 动画 spinner
- [ ] 折叠/展开模式
- [ ] Ctrl+O 切换

### Phase 3: 固定底部布局
- [x] Demo 验证可行性 (`examples/fixed_bottom_demo.rs`)
- [ ] 集成到 sage-cli
- [ ] 权限模式切换

### Phase 4: 状态栏
- [x] Demo 实现
- [ ] 集成到主应用
- [ ] 显示更多信息（会话、模型等）

### Phase 5: 交互增强
- [ ] 历史记录导航
- [ ] 多行输入
- [ ] Vim 模式

## 参考文件

- open-claude-code 源码: `/Users/apple/Desktop/code/AI/code-agent/open-claude-code/`
- 关键文件:
  - `src_v2.0.76/modules/chunk_091_config.js` - 消息渲染
  - `src_v2.0.76/modules/chunk_093_prompts.js` - Thinking 处理
  - `types/ui/render.d.ts` - 渲染函数类型定义
  - `types/ui/input.d.ts` - 输入组件类型定义
