# Edit Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Edit |
| Claude Code 版本 | 2.1.20 |
| 类别 | 文件操作 |
| Sage 实现 | `sage-tools/src/tools/file_ops/edit/` |

## 功能描述

在文件中执行精确的字符串替换操作。

## 完整 Prompt

```markdown
Performs exact string replacements in files.

Usage:
- You must use your Read tool at least once in the conversation before editing. This tool will error if you attempt an edit without reading the file.
- When editing text from Read tool output, ensure you preserve the exact indentation (tabs/spaces) as it appears AFTER the line number prefix. The line number prefix format is: spaces + line number + tab. Everything after that tab is the actual file content to match. Never include any part of the line number prefix in the old_string or new_string.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- Only use emojis if the user explicitly requests it. Avoid adding emojis to files unless asked.
- The edit will FAIL if `old_string` is not unique in the file. Either provide a larger string with more surrounding context to make it unique or use `replace_all` to change every instance of `old_string`.
- Use `replace_all` for replacing and renaming strings across the file. This parameter is useful if you want to rename a variable for instance.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| file_path | string | ✅ | 文件的绝对路径 |
| old_string | string | ✅ | 要替换的原始文本 |
| new_string | string | ✅ | 替换后的新文本 |
| replace_all | boolean | ❌ | 是否替换所有匹配项 (默认: false) |

## 设计原理

### 1. 必须先读取
**为什么**:
- 确保 Agent 了解文件当前内容
- 避免基于过时信息进行编辑
- 提供上下文以做出正确的编辑决策

### 2. 精确字符串匹配
**为什么**:
- 比行号更可靠（行号可能因其他编辑而变化）
- 强制 Agent 提供足够的上下文
- 减少错误编辑的风险

### 3. 唯一性要求
**为什么**:
- 防止意外修改多处
- 强制提供更多上下文以确保精确定位
- 如需批量替换，使用 `replace_all`

### 4. 缩进保留
**为什么**:
- Read 工具输出包含行号前缀
- Agent 必须正确解析实际内容
- 保持代码格式一致性

### 5. replace_all 选项
**为什么**:
- 支持变量重命名等批量操作
- 明确意图：单次替换 vs 全局替换
- 避免意外的全局修改

## 使用场景

### ✅ 应该使用
- 修改函数实现
- 修复 bug
- 重命名变量 (使用 replace_all)
- 更新配置值

### ❌ 不应该使用
- 创建新文件 (使用 Write)
- 大规模重写文件 (使用 Write)
- 未先读取文件

## 常见错误

### 1. old_string 不唯一
```
错误: old_string matches multiple locations in file
解决: 添加更多上下文使其唯一，或使用 replace_all
```

### 2. 包含行号前缀
```
错误: old_string not found in file
原因: 错误地包含了 "  123→" 这样的行号前缀
解决: 只使用实际文件内容
```

### 3. 缩进不匹配
```
错误: old_string not found in file
原因: 空格/制表符不匹配
解决: 精确复制 Read 输出中的缩进
```

## Sage 实现差异

Sage 的 EditTool 额外支持：
- 编辑预览模式
- 多处编辑的原子操作
- 编辑历史记录
