# Claude Code vs Sage Agent: Prompt System 深度对比分析

## 概述

本文档对比分析 Claude Code 的 prompt 系统与 Sage Agent 当前实现的差异，识别关键改进点。

---

## 1. 架构差异

### Claude Code 架构

```
system-prompts/                     # 67 个独立文件
├── system-prompt-main-system-prompt.md    # 主系统提示词
├── system-prompt-*.md                      # 其他系统提示词
├── tool-description-*.md                   # 工具描述 (18+个)
├── agent-prompt-*.md                       # 子agent提示词 (15+个)
├── system-reminder-*.md                    # 运行时提醒 (5个)
└── data-*.md                               # 数据模板
```

### Sage 当前架构

```
prompts/
├── system_prompt.rs       # 核心系统提示词
├── builder.rs             # 构建器
├── system_reminders.rs    # 运行时提醒
├── tool_descriptions/     # 工具描述
├── agent_prompts/         # 子agent提示词
└── (legacy modules)
```

### 差异分析

| 维度 | Claude Code | Sage Agent | 差距 |
|------|------------|------------|------|
| 文件数量 | 67+ | ~10 | 6x |
| 工具描述 | 18+ 独立文件 | 1个综合文件 | 需要拆分 |
| 子agent提示词 | 15+ 独立文件 | 1个综合文件 | 需要拆分 |
| 变量系统 | `${VAR}` 模板 | 硬编码 | **缺失** |

---

## 2. 变量/模板系统

### Claude Code 变量系统

```markdown
<!-- ccVersion: 2.0.68 -->
<!-- variables:
  - BASH_TOOL_NAME
  - TASK_TOOL_NAME
  - READ_TOOL_NAME
-->

Use the ${BASH_TOOL_NAME} tool to execute commands.
${AVAILABLE_TOOLS_SET.has(TODO_TOOL_OBJECT.name)?`# Task Management...`:""}
```

**特点:**
- 声明式变量定义
- 条件渲染 (`?`:`""`)
- 版本追踪
- 动态工具集检查

### Sage 当前实现

```rust
pub const BASH: &'static str = r#"Executes bash commands..."#;

pub fn build_full_prompt(identity: &str, task: &str, ...) -> String {
    format!("... {} ... {} ...", identity, task)
}
```

**特点:**
- 静态硬编码
- 简单字符串格式化
- 无条件渲染

### 需要补充

```rust
// 建议实现的变量系统
pub struct PromptVariables {
    pub bash_tool_name: String,
    pub task_tool_name: String,
    pub available_tools: HashSet<String>,
    // ... 更多变量
}

impl PromptTemplate {
    pub fn render(&self, vars: &PromptVariables) -> String {
        // 替换 ${VAR} 模式
        // 支持条件渲染
    }
}
```

---

## 3. 主系统提示词对比

### Claude Code (150+ 行)

```markdown
# Core behavior
You are an interactive CLI tool that helps users ${OUTPUT_STYLE_CONFIG}...

# Tone and style (条件渲染)
${OUTPUT_STYLE_CONFIG!==null?"":` ... 完整的风格指南 ...`}

# Professional objectivity
Prioritize technical accuracy...

# Planning without timelines
When planning tasks, provide concrete steps WITHOUT time estimates...

# Task Management (条件包含)
${AVAILABLE_TOOLS_SET.has(TODO_TOOL_OBJECT.name)?` ... TodoWrite详细指南 ...`:""}

# Asking questions (条件包含)
${AVAILABLE_TOOLS_SET.has(ASKUSERQUESTION_TOOL_NAME)?` ... 问题指南 ...`:""}

# Hook system
Users may configure 'hooks', shell commands that execute...

# Doing tasks
- NEVER propose changes to code you haven't read
- Use TodoWrite to plan if required
- Be careful not to introduce security vulnerabilities
- Avoid over-engineering...

# Tool usage policy
- Use Task tool for file search to reduce context
- Proactively use specialized agents
- Call multiple tools in parallel when possible
- Use specialized tools instead of bash commands
```

### Sage 当前 (约 80 行)

```rust
pub const CODE_FIRST_RULES: &str = r#"# CRITICAL: CODE-FIRST EXECUTION...
When users ask you to "design", "create", "implement"...
1. This ALWAYS means WRITE WORKING CODE...
"#;

pub const ROLE: &str = r#"# Role
You are Sage Agent, an agentic coding AI assistant..."#;

pub const RESPONSE_STYLE: &str = r#"# Response Style
- Be concise and direct..."#;

pub const TOOL_USAGE: &str = r#"# Tool Usage Strategy
- Use multiple tools concurrently when possible..."#;

pub const TASK_COMPLETION: &str = r#"# Task Completion Rules
You can ONLY call task_done when ALL of these are true..."#;
```

### 关键缺失

| 功能 | Claude Code | Sage |
|------|-------------|------|
| Hook系统说明 | ✅ | ❌ |
| 条件渲染 | ✅ | ❌ |
| 详细示例 | ✅ 多个 | ❌ 无 |
| Over-engineering防止 | ✅ 详细 | ❌ 简略 |
| 安全漏洞警告 | ✅ OWASP | ❌ |
| 并行工具调用 | ✅ 详细 | ✅ 简略 |

---

## 4. 工具描述对比

### Claude Code: Bash 工具 (100+ 行)

```markdown
# tool-description-bash.md

Executes a bash command in a persistent shell session...

IMPORTANT: This tool is for terminal operations like git, npm, docker...
DO NOT use it for file operations - use the specialized tools.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If creating new directories/files, first verify parent exists...

2. Command Execution:
   - Always quote file paths with spaces
   - Examples of proper quoting:
     - cd "/Users/name/My Documents" (correct)
     - cd /Users/name/My Documents (incorrect)

Usage notes:
  - Optional timeout (up to 600000ms)
  - Output truncated at 30000 chars
  - Use run_in_background parameter...

  - Avoid using find, grep, cat, head, tail, sed, awk, echo
    - File search: Use Glob (NOT find)
    - Content search: Use Grep (NOT grep)
    - Read files: Use Read (NOT cat/head/tail)
    - Edit files: Use Edit (NOT sed/awk)
    - Write files: Use Write (NOT echo)

  - When issuing multiple commands:
    - Independent: make multiple tool calls in parallel
    - Dependent: use && to chain
    - Use ';' only when don't care if earlier fails

  - Try to maintain current working directory using absolute paths
    <good-example>pytest /foo/bar/tests</good-example>
    <bad-example>cd /foo/bar && pytest tests</bad-example>
```

**额外包含:**
- Git commit 安全协议 (50+ 行)
- PR 创建指南 (30+ 行)
- HEREDOC 格式示例

### Sage: Bash 工具 (15 行)

```rust
pub const BASH: &'static str = r#"Executes bash commands in a persistent shell session.

Usage:
- For terminal operations like git, npm, docker
- Quote file paths with spaces using double quotes
- Commands timeout after 2 minutes by default

Important:
- DO NOT use for file operations - use Read/Edit/Write instead
- Avoid using grep/cat/head/tail - use specialized tools
- Can run commands in background with run_in_background parameter"#;
```

### 差距总结

| 特性 | Claude Code | Sage |
|------|-------------|------|
| 字符数 | ~4000 | ~500 |
| 示例数 | 10+ | 0 |
| Good/Bad 示例 | ✅ | ❌ |
| Git 安全协议 | ✅ 详细 | ❌ |
| PR 创建指南 | ✅ | ❌ |
| 路径引用说明 | ✅ 详细 | ✅ 简略 |
| 并行调用说明 | ✅ 详细 | ❌ |

---

## 5. Plan Mode 对比

### Claude Code: 5阶段工作流

```markdown
# system-reminder-plan-mode-is-active.md

Plan mode is active. You MUST NOT make any edits...

## Plan File Info:
${SYSTEM_REMINDER.planExists?`Plan exists at ${path}...`:`No plan file yet...`}

## Plan Workflow

### Phase 1: Initial Understanding
Goal: Comprehensive understanding of user's request
1. Focus on understanding requirements
2. **Launch up to N Explore agents IN PARALLEL**
   - Use 1 agent for isolated tasks
   - Use multiple for uncertain scope
3. Use AskUserQuestion to clarify ambiguities

### Phase 2: Design
Goal: Design implementation approach
- Launch Plan agent(s) to design based on Phase 1 results
- Can launch up to N agents in parallel for complex tasks
- Different perspectives: simplicity vs performance vs maintainability

### Phase 3: Review
Goal: Review plans and ensure alignment
1. Read critical files identified by agents
2. Ensure plans align with original request
3. Use AskUserQuestion for remaining questions

### Phase 4: Final Plan
Goal: Write final plan to plan file
- Include only recommended approach
- Concise but detailed enough to execute
- Include paths of critical files

### Phase 5: Call ExitPlanMode
- Always call at end of turn
- Your turn should only end with question or ExitPlanMode
```

### Sage: 简单布尔标记

```rust
// builder.rs
pub fn in_plan_mode(mut self, enabled: bool) -> Self {
    self.in_plan_mode = enabled;
    self
}

fn build_plan_mode_section(&self) -> String {
    if !self.in_plan_mode {
        return String::new();
    }

    r#"# PLAN MODE ACTIVE
    1. Focus on understanding and designing
    2. Use Explore and Plan agents
    3. Write plan to plan file
    4. Call ExitPlanMode when ready
    5. Keep planning brief (< 2 minutes)

    DO NOT:
    - Write code files
    - Call task_done
    - Skip to implementation"#.to_string()
}
```

```rust
// system_reminders.rs
pub enum PlanPhase {
    Understanding,
    Designing,
    Reviewing,
    Finalizing,
    Exiting,
}
// 但未实际使用这个状态机
```

### 差距

| 特性 | Claude Code | Sage |
|------|-------------|------|
| 阶段数 | 5个明确阶段 | 有定义但未使用 |
| 并行子agent | ✅ 支持N个 | ❌ |
| Plan文件状态检查 | ✅ | ❌ |
| 用户问题集成 | ✅ 每阶段 | ❌ |
| Critical Files输出 | ✅ 必须 | ❌ |
| 强制退出规则 | ✅ | ❌ |

---

## 6. 子Agent Prompt 对比

### Claude Code: Explore Agent (45 行)

```markdown
# agent-prompt-explore.md

You are a file search specialist for Claude Code...

=== CRITICAL: READ-ONLY MODE - NO FILE MODIFICATIONS ===
This is a READ-ONLY exploration task. You are STRICTLY PROHIBITED from:
- Creating new files (no Write, touch, or file creation)
- Modifying existing files (no Edit operations)
- Deleting files (no rm or deletion)
- Moving or copying files (no mv or cp)
- Creating temporary files anywhere, including /tmp
- Using redirect operators (>, >>, |) or heredocs
- Running ANY commands that change system state

Your role is EXCLUSIVELY to search and analyze existing code.
You do NOT have access to file editing tools - attempting to edit will fail.

Your strengths:
- Rapidly finding files using glob patterns
- Searching code with powerful regex
- Reading and analyzing file contents

Guidelines:
- Use Glob for broad pattern matching
- Use Grep for content search
- Use Read for specific file paths
- Use Bash ONLY for read-only operations (ls, git status, git log...)
- NEVER use Bash for: mkdir, touch, rm, cp, mv, git add, git commit...
- Adapt search approach based on thoroughness level
- Return absolute paths in final response
- Avoid emojis

NOTE: You are meant to be a FAST agent. Make efficient use of tools.
Spawn multiple parallel tool calls for grepping and reading files.

Complete the search request efficiently and report findings clearly.
```

### Sage: Explore Agent (27 行)

```rust
pub const EXPLORE: &'static str = r#"You are an Explore agent specialized in investigating codebases.

Your capabilities:
- Find files by patterns (Glob)
- Search code for keywords (Grep)
- Read and analyze files
- Understand code structure

Focus on:
- Quick, targeted searches
- Finding relevant code efficiently
- Providing concise summaries
- Identifying key files and patterns

Do NOT:
- Write or modify files
- Make implementation decisions
- Spend too much time on exploration"#;
```

### 差距

| 特性 | Claude Code | Sage |
|------|-------------|------|
| READ-ONLY 强调 | ✅ 详细列表 | ✅ 简单提及 |
| 禁止操作清单 | ✅ 7项具体 | ❌ 模糊 |
| 性能要求 | ✅ "fast agent" | ❌ |
| 并行工具调用 | ✅ 明确要求 | ❌ |
| 输出格式要求 | ✅ 绝对路径 | ❌ |
| 工具使用指南 | ✅ 具体到命令 | ✅ 简略 |

---

## 7. TodoWrite 对比

### Claude Code: TodoWrite (190 行)

```markdown
# tool-description-todowrite.md

Use this tool to create and manage a structured task list...

## When to Use This Tool
1. Complex multi-step tasks (3+ steps)
2. Non-trivial and complex tasks
3. User explicitly requests todo list
4. User provides multiple tasks
5. After receiving new instructions
6. When starting a task (mark in_progress BEFORE beginning)
7. After completing a task

## When NOT to Use This Tool
1. Single, straightforward task
2. Trivial task
3. Less than 3 trivial steps
4. Purely conversational/informational

## Examples of When to Use (4 detailed examples with <reasoning>)

<example>
User: Add a dark mode toggle...
Assistant: Creates todo list with:
1. Creating dark mode toggle component
2. Adding state management
3. Implementing CSS-in-JS styles
4. Updating existing components
5. Running tests and build

<reasoning>
1. Multi-step feature
2. User explicitly requested tests
3. Inferred tests need to pass
</reasoning>
</example>

## Examples of When NOT to Use (4 detailed examples with <reasoning>)

<example>
User: How do I print 'Hello World' in Python?
<reasoning>
Single, trivial task that can be completed in one step.
No need to track multiple tasks.
</reasoning>
</example>

## Task States and Management
1. **Task States**:
   - pending: Not yet started
   - in_progress: Currently working (limit to ONE)
   - completed: Finished successfully

   **IMPORTANT**: Task descriptions must have two forms:
   - content: Imperative form ("Run tests")
   - activeForm: Present continuous ("Running tests")

2. **Task Management**:
   - Update status in real-time
   - Mark complete IMMEDIATELY after finishing
   - Exactly ONE task in_progress at any time
   - Complete current before starting new

3. **Task Completion Requirements**:
   - ONLY mark completed when FULLY accomplished
   - If errors/blockers, keep as in_progress
   - Never mark completed if:
     - Tests are failing
     - Implementation is partial
     - Unresolved errors
     - Missing files/dependencies
```

### Sage: TodoWrite

**完全未实现** - Sage 没有 TodoWrite 工具

---

## 8. System Reminder 对比

### Claude Code: 5种 Reminder

```markdown
# system-reminder-plan-mode-is-active.md (82 行)
# system-reminder-plan-mode-is-active-for-subagents.md (30 行)
# system-reminder-plan-mode-re-entry.md (30 行)
# system-reminder-delegate-mode-prompt.md (20 行)
# system-reminder-team-coordination.md (20 行)
```

**特点:**
- 专用于特定场景
- 包含动态变量
- 结构化指令

### Sage: 5种 Reminder

```rust
pub enum SystemReminder {
    TodoListStatus { is_empty: bool, task_count: usize },
    FileOperationWarning { message: String },
    PlanModePhase { phase: PlanPhase, instructions: String },
    TaskCompletionReminder,
    Custom { title: String, content: String },
}
```

**特点:**
- 基础实现
- 静态文本
- 未与agent状态深度集成

### 差距

| 特性 | Claude Code | Sage |
|------|-------------|------|
| Reminder 类型 | 5个专用文件 | 5个枚举变体 |
| Plan Mode Reminder | ✅ 82行详细 | ✅ 15行简略 |
| Subagent Reminder | ✅ 专用 | ❌ |
| Team Coordination | ✅ | ❌ |
| 动态变量 | ✅ | ❌ |

---

## 9. 安全策略对比

### Claude Code

```markdown
# 在主系统提示词中
${SECURITY_POLICY}

# Git Safety Protocol (在 bash 工具描述中)
- NEVER update the git config
- NEVER run destructive/irreversible git commands
- NEVER skip hooks (--no-verify, --no-gpg-sign)
- NEVER run force push to main/master
- Avoid git commit --amend unless all conditions met
- CRITICAL: If commit FAILED, NEVER amend
- NEVER commit unless explicitly asked

# 在 Doing tasks 部分
- Be careful not to introduce security vulnerabilities
  such as command injection, XSS, SQL injection,
  and other OWASP top 10 vulnerabilities
```

### Sage

```rust
// 没有专门的安全策略模块
// 只有简单提及
pub const TASK_COMPLETION: &str = r#"...
- The code is functional and can be executed
..."#;
```

### 需要补充

```rust
pub struct SecurityPolicy;

impl SecurityPolicy {
    pub const GIT_SAFETY: &'static str = r#"
# Git Safety Protocol
- NEVER update git config
- NEVER run destructive git commands (push --force, hard reset)
- NEVER skip hooks (--no-verify)
- NEVER force push to main/master
- NEVER commit unless explicitly asked
"#;

    pub const CODE_SECURITY: &'static str = r#"
# Code Security
Be careful not to introduce:
- Command injection
- XSS (Cross-Site Scripting)
- SQL injection
- Other OWASP Top 10 vulnerabilities

If you notice insecure code, fix it immediately.
"#;
}
```

---

## 10. 示例系统对比

### Claude Code: 大量示例

```markdown
# 在 TodoWrite 中
4 个 "When to Use" 示例 + <reasoning>
4 个 "When NOT to Use" 示例 + <reasoning>

# 在 Task 工具中
2 个完整示例，包含:
- 用户输入
- assistant 行为
- <commentary> 解释
- 代码块

# 在 Bash 工具中
<good-example>
pytest /foo/bar/tests
</good-example>
<bad-example>
cd /foo/bar && pytest tests
</bad-example>

# 在 EnterPlanMode 中
5 个 GOOD 示例
3 个 BAD 示例
```

### Sage: 几乎没有示例

```rust
// 整个 prompts 模块中只有测试用例
// 没有对 LLM 的示例指导
```

### 需要补充

每个关键模块都需要添加:
- 2-4 个正面示例
- 2-4 个反面示例
- `<reasoning>` 标签解释原因

---

## 11. 改进优先级清单

### P0 - 必须立即修复

1. **变量/模板系统**
   - 实现 `${VAR}` 替换
   - 支持条件渲染

2. **TodoWrite 工具**
   - 完整实现 TodoWrite 工具
   - 包含详细使用指南

3. **工具描述增强**
   - Bash: 添加 Git 安全协议
   - 所有工具: 添加示例

### P1 - 高优先级

4. **Plan Mode 完整实现**
   - 使用 `PlanPhase` 状态机
   - 添加并行子agent支持
   - Plan 文件状态检查

5. **子Agent Prompt 增强**
   - 详细的 READ-ONLY 限制
   - 性能要求
   - 并行工具调用指导

6. **安全策略模块**
   - Git Safety Protocol
   - OWASP 漏洞警告

### P2 - 中优先级

7. **示例系统**
   - 为每个工具添加 good/bad 示例
   - 添加 `<reasoning>` 解释

8. **Reminder 增强**
   - Subagent 专用 reminder
   - Team coordination reminder

9. **条件渲染**
   - 根据可用工具动态生成提示词

### P3 - 低优先级

10. **Over-engineering 防止指南**
11. **文档和注释标准**
12. **版本追踪系统**

---

## 12. 总结

### 数量对比

| 指标 | Claude Code | Sage | 差距 |
|------|-------------|------|------|
| Prompt 文件数 | 67 | ~10 | 6.7x |
| 总字符数 | ~80,000 | ~8,000 | 10x |
| 示例数量 | 30+ | ~0 | ∞ |
| 工具描述 | 18个独立 | 1个综合 | - |
| 子agent提示 | 15个独立 | 1个综合 | - |

### 质量对比

| 维度 | Claude Code | Sage | 评分差 |
|------|-------------|------|--------|
| 详细程度 | ★★★★★ | ★★☆☆☆ | -3 |
| 示例丰富度 | ★★★★★ | ★☆☆☆☆ | -4 |
| 安全考虑 | ★★★★★ | ★☆☆☆☆ | -4 |
| 模块化 | ★★★★★ | ★★★☆☆ | -2 |
| 动态能力 | ★★★★★ | ★☆☆☆☆ | -4 |

### 下一步行动

1. 实现变量模板系统
2. 完成 TodoWrite 工具
3. 增强 Bash 工具描述 (添加 Git 安全协议)
4. 为所有工具添加示例
5. 完善 Plan Mode 5阶段工作流
6. 添加安全策略模块
