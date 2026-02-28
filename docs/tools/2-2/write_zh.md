# Write Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Write |
| Claude Code 版本 | 2.1.20 |
| 类别 | 文件操作 |
| Sage 实现 | `sage-tools/src/tools/file_ops/write/` |

## 功能描述

将内容写入本地文件系统，会覆盖已存在的文件。

## 完整 Prompt

```markdown
Writes a file to the local filesystem.

Usage:
- This tool will overwrite the existing file if there is one at the provided path.
- If this is an existing file, you MUST use the Read tool first to read the file's contents. This tool will fail if you did not read the file first.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| file_path | string | ✅ | 文件的绝对路径 |
| content | string | ✅ | 要写入的内容 |

## 设计原理

### 1. 必须先读取
**为什么**:
- 防止意外覆盖重要文件
- 确保 Agent 了解文件当前状态
- 避免丢失用户未提及的内容

### 2. 优先编辑而非写入
**为什么**:
- Edit 工具更精确，只修改需要的部分
- 减少意外覆盖的风险
- 保留文件的其他内容

### 3. 禁止主动创建文档
**为什么**:
- 避免生成不必要的文件
- 保持代码库整洁
- 文档应由用户明确请求

### 4. 禁止 emoji
**为什么**:
- 保持代码专业性
- 避免编码问题
- 除非用户明确要求

## 使用场景

### ✅ 应该使用
- 创建新的源代码文件
- 创建配置文件
- 用户明确要求创建新文件

### ❌ 不应该使用
- 修改现有文件 (使用 Edit)
- 主动创建 README 或文档
- 未先读取就覆盖文件

## 变量说明

| 变量 | 说明 |
|------|------|
| MUST_READ_FIRST_FN | 是否强制先读取的提示函数 |

## 安全考虑

1. **覆盖保护**: 必须先读取文件才能写入
2. **路径验证**: 只接受绝对路径
3. **内容审查**: 避免写入敏感信息

## Sage 实现差异

Sage 的 WriteTool 额外支持：
- 自动创建父目录
- 文件权限设置
- 写入前备份选项
