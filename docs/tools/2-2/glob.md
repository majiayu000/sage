# Glob Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Glob |
| Claude Code 版本 | 2.0.14 |
| 类别 | 文件操作 |
| Sage 实现 | `sage-tools/src/tools/file_ops/glob/` |

## 功能描述

快速文件模式匹配工具，支持任意大小的代码库。

## 完整 Prompt

```markdown
- Fast file pattern matching tool that works with any codebase size
- Supports glob patterns like "**/*.js" or "src/**/*.ts"
- Returns matching file paths sorted by modification time
- Use this tool when you need to find files by name patterns
- When you are doing an open ended search that may require multiple rounds of globbing and grepping, use the Agent tool instead
- You can call multiple tools in a single response. It is always better to speculatively perform multiple searches in parallel if they are potentially useful.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| pattern | string | ✅ | Glob 模式 (如 "**/*.rs") |
| path | string | ❌ | 搜索目录 (默认: 当前工作目录) |

## 设计原理

### 1. 按修改时间排序
**为什么**:
- 最近修改的文件通常最相关
- 帮助 Agent 优先关注活跃文件
- 减少处理过时文件的时间

### 2. 支持任意代码库大小
**为什么**:
- 大型项目可能有数万文件
- 需要高效的文件系统遍历
- 避免超时或内存问题

### 3. 并行搜索建议
**为什么**:
- 多个模式可以同时搜索
- 减少往返次数
- 提高整体效率

### 4. 复杂搜索使用 Agent
**为什么**:
- 开放式搜索可能需要多轮迭代
- Agent 可以自主决定搜索策略
- 避免主 Agent 上下文膨胀

## 使用场景

### ✅ 应该使用
- 查找特定类型的文件 (`**/*.ts`)
- 查找特定目录下的文件 (`src/components/**/*.tsx`)
- 查找特定命名模式的文件 (`*_test.go`)

### ❌ 不应该使用
- 搜索文件内容 (使用 Grep)
- 复杂的多轮搜索 (使用 Task/Agent)
- 列出目录结构 (使用 Bash `ls`)

## 常用模式

| 模式 | 说明 |
|------|------|
| `**/*.rs` | 所有 Rust 文件 |
| `src/**/*.ts` | src 目录下所有 TypeScript 文件 |
| `**/test_*.py` | 所有 Python 测试文件 |
| `*.{js,ts}` | 当前目录的 JS 和 TS 文件 |
| `!**/node_modules/**` | 排除 node_modules |

## Sage 实现差异

Sage 的 GlobTool 与 Claude Code 基本一致，使用 `globset` crate 实现高效匹配。
