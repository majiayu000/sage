# Read Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Read |
| Claude Code 版本 | 2.0.14 |
| 类别 | 文件操作 |
| Sage 实现 | `sage-tools/src/tools/file_ops/read/` |

## 功能描述

从本地文件系统读取文件内容，支持多种文件类型包括文本、图片、PDF 和 Jupyter notebooks。

## 完整 Prompt

```markdown
Reads a file from the local filesystem. You can access any file directly by using this tool.
Assume this tool is able to read all files on the machine. If the User provides a path to a file assume that path is valid. It is okay to read a file that does not exist; an error will be returned.

Usage:
- The file_path parameter must be an absolute path, not a relative path
- By default, it reads up to ${DEFAULT_READ_LINES} lines starting from the beginning of the file
- You can optionally specify a line offset and limit (especially handy for long files), but it's recommended to read the whole file by not providing these parameters
- Any lines longer than ${MAX_LINE_LENGTH} characters will be truncated
- Results are returned using cat -n format, with line numbers starting at 1
- This tool allows Claude Code to read images (eg PNG, JPG, etc). When reading an image file the contents are presented visually as Claude Code is a multimodal LLM.
- This tool can read PDF files (.pdf). PDFs are processed page by page, extracting both text and visual content for analysis.
- This tool can read Jupyter notebooks (.ipynb files) and returns all cells with their outputs, combining code, text, and visualizations.
- This tool can only read files, not directories. To read a directory, use an ls command via the ${BASH_TOOL_NAME} tool.
- You can call multiple tools in a single response. It is always better to speculatively read multiple potentially useful files in parallel.
- You will regularly be asked to read screenshots. If the user provides a path to a screenshot, ALWAYS use this tool to view the file at the path. This tool will work with all temporary file paths.
- If you read a file that exists but has empty contents you will receive a system reminder warning in place of file contents.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| file_path | string | ✅ | 文件的绝对路径 |
| offset | number | ❌ | 开始读取的行号 (0-indexed) |
| limit | number | ❌ | 读取的最大行数 (默认: 2000) |

## 设计原理

### 1. 绝对路径要求
**为什么**: 避免工作目录变化导致的路径解析错误，确保文件定位的确定性。

### 2. 行号格式 (cat -n)
**为什么**:
- 便于 Edit 工具精确定位修改位置
- 用户可以快速定位代码行
- 与 Unix 工具链保持一致

### 3. 多模态支持
**为什么**: Claude 是多模态 LLM，可以直接理解图片内容，无需额外的图片描述工具。

### 4. 并行读取建议
**为什么**: 减少往返次数，提高效率。当需要了解多个相关文件时，一次性读取比逐个读取更高效。

### 5. 空文件警告
**为什么**: 区分"文件不存在"和"文件存在但为空"两种情况，帮助 Agent 做出正确判断。

## 使用场景

### ✅ 应该使用
- 读取源代码文件
- 查看配置文件
- 分析日志文件
- 查看用户提供的截图
- 读取 PDF 文档
- 查看 Jupyter notebook

### ❌ 不应该使用
- 列出目录内容 (使用 Bash `ls`)
- 搜索文件内容 (使用 Grep)
- 查找文件 (使用 Glob)

## 变量说明

| 变量 | 说明 | 典型值 |
|------|------|--------|
| DEFAULT_READ_LINES | 默认读取行数 | 2000 |
| MAX_LINE_LENGTH | 最大行长度 | 2000 |
| CAN_READ_PDF_FILES | 是否支持 PDF | true/false |
| BASH_TOOL_NAME | Bash 工具名称 | "Bash" |

## Sage 实现差异

Sage 的 ReadTool 实现与 Claude Code 基本一致，额外支持：
- 自定义行号格式
- 更灵活的编码处理
- 集成安全检查
