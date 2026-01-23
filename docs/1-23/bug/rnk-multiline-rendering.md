# Bug: rnk 多行文本渲染问题

## 问题描述

在 rnk inline mode 下，多行文本内容没有正确换行显示，所有内容被水平拼接在一起。

## 复现步骤

1. 运行 `sage`
2. 输入 `/commands` 或 `/com` 然后按 Enter

## 期望行为

```
Available slash commands:

## Built-in Commands
- /status - Show agent status
- /init - Initialize Sage in project
- /output - Switch output mode
...
```

## 实际行为

```
Available slash commands: shift+tab to cycle

                         ## Built-in Commands
                                             - /status - Show agent status
                                                                          - /init - Initialize Sage in project
```

所有内容被水平拼接，没有换行。

## 根本原因分析

### 1. rnk 的 Text 组件不处理内嵌换行符

`rnk::prelude::Text::new()` 创建的文本元素不会自动将 `\n` 转换为实际的换行。在 rnk 的 inline mode 中，每个 `Text` 元素被视为单行内容。

```rust
// 这样不会换行
Text::new("line1\nline2\nline3").into_element()

// 需要这样才能换行
RnkBox::new()
    .flex_direction(FlexDirection::Column)
    .child(Text::new("line1").into_element())
    .child(Text::new("line2").into_element())
    .child(Text::new("line3").into_element())
```

### 2. 受影响的代码位置

#### a) `/commands` 输出 (`sage-core/src/commands/executor/handlers/basic.rs`)

```rust
pub(super) async fn execute_commands(executor: &CommandExecutor) -> SageResult<CommandResult> {
    // ...
    let mut output = String::from("Available slash commands:\n\n");
    // ... 构建包含 \n 的字符串
    Ok(CommandResult::local(output))  // 这个 output 包含 \n，但不会被正确渲染
}
```

#### b) 消息格式化 (`sage-cli/src/ui/rnk_app/components.rs`)

```rust
pub fn format_message(msg: &Message) -> Element {
    match &msg.content {
        MessageContent::Text(text) => {
            // 这里尝试处理换行，但方式可能不正确
            for line in text.lines() {
                let wrapped = wrap_text_with_prefix("", line, term_width);
                // ...
            }
        }
    }
}
```

### 3. rnk::println vs 标准 println

- `rnk::println(element)` - 打印一个 rnk Element，用于 inline mode
- 标准 `println!()` - 直接打印到终端，会破坏 rnk 的布局

在 rnk inline mode 中，所有输出都应该通过 `rnk::println()` 进行，但这要求内容必须是正确构建的 Element 树。

## 解决方案

### 方案 1: 将多行字符串转换为 Column 布局

```rust
fn multiline_to_element(text: &str) -> Element {
    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
    for line in text.lines() {
        container = container.child(
            Text::new(line).into_element()
        );
    }
    container.into_element()
}
```

### 方案 2: 修改 CommandResult 处理

在 executor 中处理 `CommandResult::Local` 时，将文本按行分割并逐行打印：

```rust
// 在 executor.rs 中
if let SlashCommandAction::Handled = action {
    // 如果有本地输出，按行打印
    for line in output.lines() {
        rnk::println(Text::new(line).into_element());
    }
}
```

### 方案 3: 使用 rnk 的原生换行支持（如果有）

检查 rnk 是否有内置的多行文本支持，如 `Text::new().multiline(true)` 或类似 API。

## 相关文件

| 文件 | 职责 |
|------|------|
| `sage-cli/src/ui/rnk_app/executor.rs` | 命令执行和输出 |
| `sage-cli/src/ui/rnk_app/components.rs` | UI 组件渲染 |
| `sage-core/src/commands/executor/handlers/basic.rs` | 命令处理器 |

## 优先级

**P1** - 影响基本功能使用，命令输出无法阅读

## 修复建议

1. 创建一个通用的 `text_to_element(text: &str) -> Element` 函数
2. 在所有需要显示多行文本的地方使用这个函数
3. 确保 `CommandResult::Local` 的输出被正确转换为 Element 树

## 测试用例

```rust
#[test]
fn test_multiline_rendering() {
    let text = "line1\nline2\nline3";
    let element = text_to_element(text);
    // 验证 element 是一个包含 3 个子元素的 Column 布局
}
```
