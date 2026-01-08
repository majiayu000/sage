# Sage Agent 工具 Prompt 对比分析 (剩余工具)

## 对比状态表

| 工具 | Claude Code | Sage Agent | 状态 | 差距 |
|------|-------------|------------|------|------|
| **Bash** | ✅ 完整 (1074+ tokens) | ✅ 已更新 | ✅ 对齐 | - |
| **Grep** | ✅ 完整 | ✅ 已更新 | ✅ 对齐 | - |
| **Read** | ✅ 完整 | ✅ 已更新 | ✅ 对齐 | - |
| **Edit** | ✅ 完整 | ✅ 已更新 | ✅ 对齐 | - |
| **Glob** | ✅ 完整 | ✅ 已更新 | ✅ 对齐 | - |
| **Write** | ✅ 完整 | ✅ 基本一致 | ✅ OK | 微小差异 |
| **NotebookEdit** | ✅ 完整 | ✅ 基本一致 | ✅ OK | 微小差异 |
| **WebFetch** | ✅ 完整 (详细) | ⚠️ 简单 | ⚠️ 需改进 | 缺少使用说明 |
| **WebSearch** | ✅ 完整 (含 Sources 要求) | ⚠️ 简单 | ⚠️ 需改进 | 缺少 Sources 要求 |
| **Task** | ✅ 完整 (多 agent 类型) | ✅ 较完整 | ⚠️ 可改进 | 缺少示例 |
| **TodoWrite** | ✅ 超详细 (2167 tokens) | ⚠️ 简单 | ⚠️ 需大改进 | 缺少示例和指南 |
| **AskUserQuestion** | ✅ 完整 | ✅ 基本一致 | ✅ OK | 微小差异 |
| **EnterPlanMode** | ✅ 完整 (含示例) | ⚠️ 简单 | ⚠️ 需改进 | 缺少 7 种使用场景 |
| **ExitPlanMode** | ✅ 完整 | ❓ 需检查 | - | - |
| **LSP** | ✅ 完整 | ❌ 无此工具 | ❌ 缺失 | - |
| **Skill** | ✅ 完整 | ❓ 需检查 | - | - |
| **KillShell** | ✅ 有 | ✅ 有 | ✅ OK | - |
| **TaskOutput** | ✅ 有 | ✅ 有 | ✅ OK | - |

---

## 详细对比

### 1. Write 工具 - ✅ OK

**Claude Code:**
```
Writes a file to the local filesystem.

Usage:
- This tool will overwrite the existing file if there is one at the provided path.
- If this is an existing file, you MUST use the Read tool first to read the file's contents.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.
- Only use emojis if the user explicitly requests it.
```

**Sage Agent:**
```
基本相同，额外增加了 Security 部分说明
```

**差异:** 无需改进

---

### 2. NotebookEdit 工具 - ✅ OK

**Claude Code:**
```
Completely replaces the contents of a specific cell in a Jupyter notebook (.ipynb file) with new source.
Jupyter notebooks are interactive documents that combine code, text, and visualizations, commonly used
for data analysis and scientific computing. The notebook_path parameter must be an absolute path,
not a relative path. The cell_number is 0-indexed. Use edit_mode=insert to add a new cell at the
index specified by cell_number. Use edit_mode=delete to delete the cell at the index specified
by cell_number.
```

**Sage Agent:**
```
Edit Jupyter notebook (.ipynb) cells. Supports three operations:
- replace: Replace the content of an existing cell (requires cell_id)
- insert: Insert a new cell at a position (requires cell_type; inserts after cell_id if provided, or at beginning)
- delete: Delete an existing cell (requires cell_id)

The notebook_path must be an absolute path to a .ipynb file.
Cell IDs can be found by reading the notebook file first.
```

**差异:** Sage 版本更清晰，无需改进

---

### 3. WebFetch 工具 - ⚠️ 需改进

**Claude Code (详细):**
```
- Fetches content from a specified URL and processes it using an AI model
- Takes a URL and a prompt as input
- Fetches the URL content, converts HTML to markdown
- Processes the content with the prompt using a small, fast model
- Returns the model's response about the content
- Use this tool when you need to retrieve and analyze web content

Usage notes:
  - IMPORTANT: If an MCP-provided web fetch tool is available, prefer using that tool instead
  - The URL must be a fully-formed valid URL
  - HTTP URLs will be automatically upgraded to HTTPS
  - The prompt should describe what information you want to extract from the page
  - This tool is read-only and does not modify any files
  - Results may be summarized if the content is very large
  - Includes a self-cleaning 15-minute cache for faster responses
  - When a URL redirects to a different host, the tool will inform you and provide the redirect URL
```

**Sage Agent (简单):**
```
Fetches data from a webpage and converts it into Markdown.
1. The tool takes in a URL and returns the content of the page in Markdown format;
2. If the return is not valid Markdown, it means the tool cannot successfully parse this page.
```

**需要添加:**
- 缓存说明
- 重定向处理说明
- MCP 工具优先说明
- HTTP -> HTTPS 自动升级说明

---

### 4. WebSearch 工具 - ⚠️ 需改进

**Claude Code (详细):**
```
- Allows Claude to search the web and use the results to inform responses
- Provides up-to-date information for current events and recent data
- Returns search result information formatted as search result blocks, including links as markdown hyperlinks
- Use this tool for accessing information beyond Claude's knowledge cutoff
- Searches are performed automatically within a single API call

CRITICAL REQUIREMENT - You MUST follow this:
  - After answering the user's question, you MUST include a "Sources:" section at the end of your response
  - In the Sources section, list all relevant URLs from the search results as markdown hyperlinks: [Title](URL)
  - This is MANDATORY - never skip including sources in your response
  - Example format:

    [Your answer here]

    Sources:
    - [Source Title 1](https://example.com/1)
    - [Source Title 2](https://example.com/2)

Usage notes:
  - Domain filtering is supported to include or block specific websites

IMPORTANT - Use the correct year in search queries:
  - Today's date is ${GET_CURRENT_DATE_FN()}. You MUST use this year when searching for recent information
```

**Sage Agent (简单):**
```
Search the web for information. Returns results in markdown format.
IMPORTANT: If search returns placeholder results or fails, DO NOT retry indefinitely.
Instead, use your built-in knowledge to proceed with the task.
```

**需要添加:**
- **Sources 要求** (关键!)
- 日期年份要求
- 域名过滤说明

---

### 5. Task 工具 - ⚠️ 可改进

**Claude Code (详细):**
```
Launch a new agent to handle complex, multi-step tasks autonomously.

Available agent types and the tools they have access to:
${AGENT_TYPE_REGISTRY_STRING}

When NOT to use the Task tool:
- If you want to read a specific file path, use Read or Glob tool instead
- If you are searching for a specific class definition, use Glob tool instead
- If you are searching for code within 2-3 files, use Read tool instead

Usage notes:
- Always include a short description (3-5 words)
- Launch multiple agents concurrently whenever possible
- Agent results are not visible to the user - summarize results in your response
- Use run_in_background parameter for background execution
- Agents can be resumed using the `resume` parameter
- Provide clear, detailed prompts
- Tell the agent whether to write code or just research

Example usage:
<example>
user: "Please write a function that checks if a number is prime"
assistant: [Uses Task tool to launch code-reviewer agent after writing code]
</example>
```

**Sage Agent:**
```
基本包含主要内容，但缺少详细示例
```

**需要添加:**
- 详细使用示例
- "Tell the agent whether to write code or just research" 说明

---

### 6. TodoWrite 工具 - ⚠️ 需大改进

**Claude Code (2167 tokens!):**
包含:
1. **When to Use** (7 种场景)
2. **When NOT to Use** (4 种场景)
3. **4 个正面示例** (详细对话)
4. **4 个负面示例** (详细对话)
5. **Task States** (详细说明)
6. **Task Management** (4 条规则)
7. **Task Completion Requirements** (4 条规则)
8. **Task Breakdown** (4 条规则)

**Sage Agent (简单):**
```
Create and manage a structured task list for your current coding session.
This helps you track progress, organize complex tasks, and demonstrate
thoroughness to the user.
```

**需要添加:**
- 7 种使用场景
- 4 种不使用场景
- 正面/负面示例
- 详细的任务管理规则

---

### 7. EnterPlanMode 工具 - ⚠️ 需改进

**Claude Code (详细):**
```
Use this tool proactively when you're about to start a non-trivial implementation task.

## When to Use This Tool

1. **New Feature Implementation**: Adding meaningful new functionality
2. **Multiple Valid Approaches**: The task can be solved in several different ways
3. **Code Modifications**: Changes that affect existing behavior or structure
4. **Architectural Decisions**: The task requires choosing between patterns or technologies
5. **Multi-File Changes**: The task will likely touch more than 2-3 files
6. **Unclear Requirements**: You need to explore before understanding the full scope
7. **User Preferences Matter**: The implementation could reasonably go multiple ways

## When NOT to Use This Tool
- Single-line or few-line fixes
- Adding a single function with clear requirements
- Tasks where the user has given very specific instructions
- Pure research/exploration tasks

## What Happens in Plan Mode
1. Thoroughly explore the codebase using Glob, Grep, and Read tools
2. Understand existing patterns and architecture
3. Design an implementation approach
4. Present your plan to the user for approval
5. Use AskUserQuestion if you need to clarify approaches
6. Exit plan mode with ExitPlanMode when ready to implement

## Examples
[多个 GOOD 和 BAD 示例]
```

**Sage Agent (简单):**
```
Enter QUICK plan mode for brief analysis before coding. Use sparingly - most tasks
should start with code immediately. Plan mode is ONLY for complex multi-component tasks.
Keep planning under 2 minutes, then exit and START WRITING CODE.
Do NOT use plan mode for simple features or bug fixes.
```

**需要添加:**
- 7 种使用场景
- 4 种不使用场景
- Plan Mode 步骤说明
- 具体示例

---

## 优先级排序

### 高优先级 (影响 agent 行为)

1. **TodoWrite** - 缺少详细使用指南和示例
2. **EnterPlanMode** - 缺少使用场景和示例
3. **WebSearch** - 缺少 Sources 要求 (重要!)

### 中优先级

4. **WebFetch** - 缺少详细使用说明
5. **Task** - 缺少详细示例

### 低优先级

6. 其他工具已基本对齐

---

## 下一步行动

1. 更新 TodoWrite prompt - 添加完整的使用指南和示例
2. 更新 EnterPlanMode prompt - 添加 7 种使用场景
3. 更新 WebSearch prompt - 添加 Sources 要求
4. 更新 WebFetch prompt - 添加详细使用说明
5. 更新 Task prompt - 添加详细示例
