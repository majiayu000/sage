# Claude Code vs Sage Agent 工具系统对比分析

## 数据来源

- **Claude Code Prompts**: [Piebald-AI/claude-code-system-prompts](https://github.com/Piebald-AI/claude-code-system-prompts) (v2.0.76)
- **Sage Agent**: 本地代码库

---

## 1. 系统 Prompt 对比

### Claude Code 主系统 Prompt (2981 tokens)

关键指令：
```
1. 工具使用策略
   - 文件搜索优先使用 Task tool 减少 context
   - 主动使用 Task 子代理匹配任务
   - 并行调用无依赖的工具
   - 专用工具代替 bash (Read 代替 cat, Edit 代替 sed)
   - 代码库探索必须使用 Task + Explore agent

2. 任务执行指南
   - 永远不要对未读过的代码提出修改
   - 使用 TodoWrite 规划任务
   - 使用 AskUserQuestion 澄清问题
   - 避免引入安全漏洞 (OWASP top 10)
   - 避免过度工程 - 只做明确要求的事

3. 代码风格
   - 不添加多余功能、重构或"改进"
   - 不添加不必要的错误处理
   - 不创建一次性操作的辅助函数
   - 不设计假设的未来需求
```

### Sage Agent 系统 Prompt

- 基础系统提示
- 缺少详细的工具使用策略
- 缺少任务执行指南

---

## 2. 工具 Prompt 详细对比

### 2.1 Bash 工具

#### Claude Code (1074 tokens)

```markdown
Executes a given bash command in a persistent shell session with optional
timeout, ensuring proper handling and security measures.

IMPORTANT: This tool is for terminal operations like git, npm, docker, etc.
DO NOT use it for file operations (reading, writing, editing, searching,
finding files) - use the specialized tools for this instead.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If the command will create new directories or files, first use `ls` to
     verify the parent directory exists and is the correct location

2. Command Execution:
   - Always quote file paths that contain spaces with double quotes
   - Examples of proper quoting:
     - cd "/Users/name/My Documents" (correct)
     - cd /Users/name/My Documents (incorrect - will fail)

Usage notes:
  - Timeout configurable (up to 10 minutes, default 2 minutes)
  - Output truncated at 30000 characters
  - run_in_background for async execution

  - Avoid using Bash with: find, grep, cat, head, tail, sed, awk, echo
    - File search: Use Glob (NOT find or ls)
    - Content search: Use Grep (NOT grep or rg)
    - Read files: Use Read (NOT cat/head/tail)
    - Edit files: Use Edit (NOT sed/awk)
    - Write files: Use Write (NOT echo >/cat <<EOF)
    - Communication: Output text directly (NOT echo/printf)

  - When issuing multiple commands:
    - Independent: make multiple Bash tool calls in parallel
    - Dependent: use '&&' to chain
    - Use ';' when don't care about failure
    - DO NOT use newlines to separate commands

  - Maintain working directory using absolute paths

# Git Commit Instructions (完整流程):
  Git Safety Protocol:
  - NEVER update git config
  - NEVER run destructive commands unless user explicitly requests
  - NEVER skip hooks unless user explicitly requests
  - NEVER force push to main/master
  - Avoid git commit --amend (严格条件才能使用)

  Commit Steps:
  1. 并行运行 git status, git diff, git log
  2. 分析变更，草拟 commit message
  3. 并行: 添加文件, 创建 commit, 验证成功
  4. 如果 pre-commit hook 失败，修复后创建新 commit

# Creating Pull Requests (完整流程):
  1. 并行运行: git status, git diff, 检查远程分支, git log
  2. 分析所有要包含的 commits
  3. 并行: 创建分支, push, 使用 gh pr create
```

#### Sage Agent (约 400 tokens)

```markdown
Execute bash commands in the shell. Use this tool to run system commands,
file operations, and other shell tasks.

Parameters:
- command: The bash command to execute
- run_in_background: If true, run command in background (default: false)
- shell_id: Optional custom ID for background shell

IMPORTANT: Avoid commands that produce excessive output:
- Use 'find . -name "*.rs" | head -20' instead of 'find . -name "*.rs"'
- Use 'ls -la | head -10' instead of 'ls -R'
- Use 'grep -n pattern file | head -10' for searches
- Always limit output with 'head', 'tail', or line count limits

Background mode:
When run_in_background=true, the command starts and returns immediately.
Use task_output(shell_id) to retrieve output.
```

**差异分析:**

| 特性 | Claude Code | Sage Agent |
|------|-------------|------------|
| 工具选择指南 | 明确列出 5 种替代 | 无 |
| 多命令执行 | 详细并行/串行建议 | 无 |
| Git Commit 流程 | 完整 5 步骤流程 | 无 |
| PR 创建流程 | 完整 3 步骤流程 | 无 |
| 路径引号示例 | 正确/错误对比 | 无 |
| 目录验证 | 建议先 ls 验证 | 无 |
| 安全协议 | 6 条 Git 安全规则 | 无 |
| Prompt 长度 | ~1074 tokens | ~400 tokens |

### 2.2 Edit 工具

#### Claude Code

```markdown
Performs exact string replacements in files.

Usage:
- You must use your `Read` tool at least once in the conversation before editing.
  This tool will error if you attempt an edit without reading the file.
- When editing text from Read tool output, ensure you preserve the exact
  indentation (tabs/spaces) as it appears AFTER the line number prefix.
  The line number prefix format is: spaces + line number + tab.
  Everything after that tab is the actual file content to match.
  Never include any part of the line number prefix in the old_string or new_string.
- ALWAYS prefer editing existing files in the codebase.
  NEVER write new files unless explicitly required.
- Only use emojis if the user explicitly requests it.
- The edit will FAIL if `old_string` is not unique in the file.
  Either provide a larger string with more surrounding context to make it unique
  or use `replace_all` to change every instance of `old_string`.
- Use `replace_all` for replacing and renaming strings across the file.
```

#### Sage Agent

```markdown
Performs exact string replacements in files.

Usage:
- You must use your `Read` tool at least once in the conversation before editing.
- When editing text from Read tool output, ensure you preserve the exact
  indentation (tabs/spaces) as it appears AFTER the line number prefix.
- ALWAYS prefer editing existing files in the codebase.
  NEVER write new files unless explicitly required.
- Only use emojis if the user explicitly requests it.
- The edit will FAIL if `old_string` is not unique in the file. To fix this:
  1. Provide MORE CONTEXT: Include surrounding lines
  2. Or use `replace_all=true` if you want to change ALL occurrences
- Use `replace_all` for replacing and renaming strings across the file.
```

**差异:** 基本一致，Sage 版本稍微扩展了错误处理说明。

### 2.3 Grep 工具

#### Claude Code

```markdown
A powerful search tool built on ripgrep

Usage:
- ALWAYS use Grep for search tasks. NEVER invoke `grep` or `rg` as a Bash command.
  The Grep tool has been optimized for correct permissions and access.
- Supports full regex syntax (e.g., "log.*Error", "function\\s+\\w+")
- Filter files with glob parameter (e.g., "*.js", "**/*.tsx") or
  type parameter (e.g., "js", "py", "rust")
- Output modes: "content" shows matching lines, "files_with_matches" shows
  only file paths (default), "count" shows match counts
- Use Task tool for open-ended searches requiring multiple rounds
- Pattern syntax: Uses ripgrep (not grep) - literal braces need escaping
- Multiline matching: By default patterns match within single lines only.
  For cross-line patterns, use `multiline: true`
```

#### Sage Agent

需要检查当前实现...

### 2.4 TodoWrite 工具 (2167 tokens)

Claude Code 的 TodoWrite 是最长的工具 prompt，包含:

1. **何时使用** (7 种场景)
   - 复杂多步骤任务 (3+ 步骤)
   - 非平凡任务
   - 用户明确要求
   - 用户提供多个任务
   - 收到新指令后
   - 开始任务时标记 in_progress
   - 完成任务后标记 completed

2. **何时不使用** (4 种场景)
   - 单一简单任务
   - 平凡任务
   - 少于 3 步
   - 纯信息性请求

3. **4 个正面示例**
   - Dark mode toggle 实现
   - 重命名函数跨项目
   - 电商功能实现
   - React 性能优化

4. **4 个负面示例**
   - Hello World 打印
   - git status 解释
   - 添加单个注释
   - npm install 执行

5. **任务状态管理**
   - pending, in_progress, completed
   - 每个任务需要 content 和 activeForm 两种形式
   - 实时更新状态
   - 一次只能有一个 in_progress

---

## 3. 关键差异总结

### 3.1 Prompt 设计差异

| 方面 | Claude Code | Sage Agent | 影响 |
|------|-------------|------------|------|
| **总 Token 数** | ~15000+ | ~3000 | 指导详细程度 |
| **工具选择指南** | 每个工具明确何时用其他工具 | 无 | Agent 可能选错工具 |
| **Git 操作** | 完整 commit/PR 工作流 + 安全协议 | 无 | Agent 不知如何提交 |
| **多命令执行** | 详细并行/串行建议 | 无 | 执行效率低 |
| **错误预防** | 大量示例和边界情况 | 基础 | 更多执行错误 |
| **TodoWrite** | 2167 tokens 详细指南 | 无此工具 | 任务规划能力差 |

### 3.2 缺失的关键 Prompt 内容

Sage Agent 需要添加:

1. **Bash 工具**
   - 工具选择指南 (何时用 Grep/Read 替代 grep/cat)
   - Git commit 完整流程 + 安全协议
   - PR 创建流程
   - 多命令执行建议

2. **系统 Prompt**
   - 代码库探索必须用 Task + Explore
   - 并行工具调用策略
   - 避免过度工程的指南

3. **新工具**
   - TodoWrite (任务管理)
   - AskUserQuestion (澄清问题)

---

## 4. 改进建议

### 高优先级

1. **更新 Bash prompt**
   ```rust
   // 添加工具选择指南
   "- Avoid using Bash with: find, grep, cat, head, tail, sed, awk, echo
     - File search: Use Glob (NOT find or ls)
     - Content search: Use Grep (NOT grep or rg)
     - Read files: Use Read (NOT cat/head/tail)
     - Edit files: Use Edit (NOT sed/awk)
     - Write files: Use Write (NOT echo >/cat <<EOF)"
   ```

2. **添加 Git 操作指南到 Bash prompt**
   - 完整的 commit 流程
   - 安全协议
   - PR 创建流程

3. **实现 TodoWrite 工具**
   - 任务状态管理
   - 详细的使用指南

### 中优先级

1. **更新系统 prompt**
   - 添加工具使用策略
   - 添加任务执行指南
   - 添加避免过度工程指南

2. **增强 Grep prompt**
   - 强调不要用 bash grep
   - 添加 multiline 说明

### 低优先级

1. **添加更多示例**
   - 每个工具添加正面/负面示例
   - 类似 TodoWrite 的详细示例

---

## 5. 参考文件

### Claude Code (claude-code-system-prompts)
- 主系统 prompt: `system-prompt-main-system-prompt.md`
- Bash: `tool-description-bash.md` + `tool-description-bash-git-commit-and-pr-creation-instructions.md`
- Edit: `tool-description-edit.md`
- Grep: `tool-description-grep.md`
- TodoWrite: `tool-description-todowrite.md`
- Read: `tool-description-readfile.md`
- Task: `tool-description-task.md`

### Sage Agent
- Edit: `crates/sage-tools/src/tools/file_ops/edit.rs`
- Bash: `crates/sage-tools/src/tools/process/bash/mod.rs`
- Grep: `crates/sage-tools/src/tools/file_ops/grep/`
- Read: `crates/sage-tools/src/tools/file_ops/read/`
