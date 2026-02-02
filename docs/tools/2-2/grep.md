# Grep Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Grep |
| Claude Code 版本 | 2.0.14 |
| 类别 | 文件操作 |
| Sage 实现 | `sage-tools/src/tools/file_ops/grep/` |

## 功能描述

基于 ripgrep 的强大内容搜索工具，支持正则表达式和多种输出模式。

## 完整 Prompt

```markdown
A powerful search tool built on ripgrep

  Usage:
  - ALWAYS use ${GREP_TOOL_NAME} for search tasks. NEVER invoke `grep` or `rg` as a ${BASH_TOOL_NAME} command. The ${GREP_TOOL_NAME} tool has been optimized for correct permissions and access.
  - Supports full regex syntax (e.g., "log.*Error", "function\\s+\\w+")
  - Filter files with glob parameter (e.g., "*.js", "**/*.tsx") or type parameter (e.g., "js", "py", "rust")
  - Output modes: "content" shows matching lines, "files_with_matches" shows only file paths (default), "count" shows match counts
  - Use ${TASK_TOOL_NAME} tool for open-ended searches requiring multiple rounds
  - Pattern syntax: Uses ripgrep (not grep) - literal braces need escaping (use `interface\{\}` to find `interface{}` in Go code)
  - Multiline matching: By default patterns match within single lines only. For cross-line patterns like `struct \{[\s\S]*?field`, use `multiline: true`
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| pattern | string | ✅ | 正则表达式搜索模式 |
| path | string | ❌ | 搜索路径 (默认: 当前目录) |
| glob | string | ❌ | 文件过滤模式 (如 "*.js") |
| type | string | ❌ | 文件类型 (如 "js", "py", "rust") |
| output_mode | string | ❌ | 输出模式: "content", "files_with_matches" (默认), "count" |
| -A | number | ❌ | 显示匹配后的行数 |
| -B | number | ❌ | 显示匹配前的行数 |
| -C | number | ❌ | 显示匹配前后的行数 |
| -i | boolean | ❌ | 忽略大小写 |
| -n | boolean | ❌ | 显示行号 (默认: true) |
| multiline | boolean | ❌ | 启用多行匹配模式 |
| head_limit | number | ❌ | 限制输出条目数 |
| offset | number | ❌ | 跳过前 N 条结果 |

## 设计原理

### 1. 禁止使用 Bash grep/rg
**为什么**:
- Grep 工具已针对权限和访问进行优化
- 避免 shell 转义问题
- 统一的输出格式便于解析

### 2. 基于 ripgrep
**为什么**:
- 比传统 grep 快 10-100 倍
- 默认忽略 .gitignore 中的文件
- 更好的 Unicode 支持

### 3. 多种输出模式
**为什么**:
- `files_with_matches`: 快速定位文件
- `content`: 查看具体匹配内容
- `count`: 了解匹配规模

### 4. 文件类型过滤
**为什么**:
- 比 glob 更高效
- 内置常见语言类型
- 减少无关文件的搜索

### 5. 多行匹配
**为什么**:
- 支持跨行的代码模式
- 如函数定义、结构体等
- 需要显式启用以避免性能问题

## 使用场景

### ✅ 应该使用
- 搜索函数定义 (`fn\s+\w+`)
- 查找特定字符串 (`TODO:`)
- 搜索导入语句 (`import.*react`)
- 查找错误处理 (`catch|except`)

### ❌ 不应该使用
- 查找文件名 (使用 Glob)
- 读取文件内容 (使用 Read)
- 复杂的多轮搜索 (使用 Task)

## 常用模式示例

| 目的 | 模式 |
|------|------|
| 函数定义 | `fn\s+(\w+)` |
| 类定义 | `class\s+\w+` |
| TODO 注释 | `TODO:?\s*` |
| 导入语句 | `^import\s+` |
| 错误处理 | `(catch|except|Error)` |
| Go 接口 | `interface\{\}` (需转义) |

## Sage 实现差异

Sage 的 GrepTool 使用 `grep` crate 封装 ripgrep，提供：
- 相同的参数接口
- 集成安全路径检查
- 结果缓存优化
