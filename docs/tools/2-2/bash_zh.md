# Bash Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Bash |
| Claude Code 版本 | 2.1.14 |
| 类别 | 执行工具 |
| Sage 实现 | `sage-tools/src/tools/process/bash/` |

## 功能描述

执行 shell 命令，支持超时控制和后台运行。工作目录在命令间持久化，但 shell 状态不会保留。

## 完整 Prompt

```markdown
Executes a given bash command with optional timeout. Working directory persists between commands; shell state (everything else) does not. The shell environment is initialized from the user's profile (bash or zsh).

IMPORTANT: This tool is for terminal operations like git, npm, docker, etc. DO NOT use it for file operations (reading, writing, editing, searching, finding files) - use the specialized tools for this instead.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If the command will create new directories or files, first use `ls` to verify the parent directory exists and is the correct location
   - For example, before running "mkdir foo/bar", first use `ls foo` to check that "foo" exists and is the intended parent directory

2. Command Execution:
   - Always quote file paths that contain spaces with double quotes (e.g., cd "path with spaces/file.txt")
   - Examples of proper quoting:
     - cd "/Users/name/My Documents" (correct)
     - cd /Users/name/My Documents (incorrect - will fail)
     - python "/path/with spaces/script.py" (correct)
     - python /path/with spaces/script.py (incorrect - will fail)
   - After ensuring proper quoting, execute the command.
   - Capture the output of the command.

Usage notes:
  - The command argument is required.
  - You can specify an optional timeout in milliseconds (up to ${CUSTOM_TIMEOUT_MS}ms). If not specified, commands will timeout after ${MAX_TIMEOUT_MS}ms.
  - It is very helpful if you write a clear, concise description of what this command does. For simple commands, keep it brief (5-10 words). For complex commands (piped commands, obscure flags, or anything hard to understand at a glance), add enough context to clarify what it does.
  - If the output exceeds ${MAX_OUTPUT_CHARS} characters, output will be truncated before being returned to you.
  - You can use the `run_in_background` parameter to run the command in the background.
  - Avoid using Bash with the `find`, `grep`, `cat`, `head`, `tail`, `sed`, `awk`, or `echo` commands, unless explicitly instructed or when these commands are truly necessary for the task. Instead, always prefer using the dedicated tools for these commands:
    - File search: Use ${GLOB_TOOL_NAME} (NOT find or ls)
    - Content search: Use ${GREP_TOOL_NAME} (NOT grep or rg)
    - Read files: Use ${READ_TOOL_NAME} (NOT cat/head/tail)
    - Edit files: Use ${EDIT_TOOL_NAME} (NOT sed/awk)
    - Write files: Use ${WRITE_TOOL_NAME} (NOT echo >/cat <<EOF)
    - Communication: Output text directly (NOT echo/printf)
  - When issuing multiple commands:
    - If the commands are independent and can run in parallel, make multiple ${BASH_TOOL_NAME} tool calls in a single message.
    - If the commands depend on each other and must run sequentially, use a single ${BASH_TOOL_NAME} call with '&&' to chain them together.
    - Use ';' only when you need to run commands sequentially but don't care if earlier commands fail
    - DO NOT use newlines to separate commands (newlines are ok in quoted strings)
  - Try to maintain your current working directory throughout the session by using absolute paths and avoiding usage of `cd`.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| command | string | ✅ | 要执行的命令 |
| description | string | ❌ | 命令描述 (推荐) |
| timeout | number | ❌ | 超时时间 (毫秒) |
| run_in_background | boolean | ❌ | 是否后台运行 |

## 设计原理

### 1. 禁止文件操作命令
**为什么**:
- 专用工具更安全、更可靠
- 避免 shell 转义问题
- 统一的输出格式

**禁止的命令**:
| 命令 | 替代工具 |
|------|----------|
| find, ls | Glob |
| grep, rg | Grep |
| cat, head, tail | Read |
| sed, awk | Edit |
| echo >, cat <<EOF | Write |

### 2. 目录验证
**为什么**:
- 防止在错误位置创建文件
- 避免覆盖重要目录
- 提前发现路径问题

### 3. 路径引号
**为什么**:
- 空格路径是常见错误来源
- 强制良好习惯
- 避免命令解析错误

### 4. 命令描述
**为什么**:
- 帮助用户理解操作
- 便于审计和回溯
- 复杂命令需要解释

### 5. 使用绝对路径
**为什么**:
- 避免 cd 导致的状态混乱
- 命令更加明确
- 减少错误

## 使用场景

### ✅ 应该使用
- Git 操作 (`git status`, `git commit`)
- 包管理 (`npm install`, `cargo build`)
- 运行测试 (`pytest`, `cargo test`)
- Docker 操作 (`docker build`)
- 系统命令 (`which`, `env`)

### ❌ 不应该使用
- 读取文件 (使用 Read)
- 搜索文件 (使用 Glob/Grep)
- 编辑文件 (使用 Edit)
- 与用户通信 (直接输出文本)

## 命令链接

```bash
# 并行执行 (多个 Bash 调用)
git status  # 调用 1
git diff    # 调用 2

# 顺序执行 (单个调用)
git add . && git commit -m "message" && git push

# 忽略失败的顺序执行
command1 ; command2 ; command3
```

## Sage 实现差异

Sage 的 BashTool 额外支持：
- 持久化 shell 会话
- 命令历史记录
- 危险命令确认
- 沙箱模式
