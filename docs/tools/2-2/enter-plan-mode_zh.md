# EnterPlanMode Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | EnterPlanMode |
| Claude Code 版本 | 2.0.62 |
| 类别 | 规划工具 |
| Sage 实现 | `sage-tools/src/tools/planning/enter_plan_mode/` |

## 功能描述

进入规划模式，在编写代码前探索代码库并设计实现方案，获取用户批准。

## 完整 Prompt

```markdown
Use this tool proactively when you're about to start a non-trivial implementation task. Getting user sign-off on your approach before writing code prevents wasted effort and ensures alignment. This tool transitions you into plan mode where you can explore the codebase and design an implementation approach for user approval.

## When to Use This Tool

**Prefer using EnterPlanMode** for implementation tasks unless they're simple. Use it when ANY of these conditions apply:

1. **New Feature Implementation**: Adding meaningful new functionality
   - Example: "Add a logout button" - where should it go? What should happen on click?
   - Example: "Add form validation" - what rules? What error messages?

2. **Multiple Valid Approaches**: The task can be solved in several different ways
   - Example: "Add caching to the API" - could use Redis, in-memory, file-based, etc.
   - Example: "Improve performance" - many optimization strategies possible

3. **Code Modifications**: Changes that affect existing behavior or structure
   - Example: "Update the login flow" - what exactly should change?
   - Example: "Refactor this component" - what's the target architecture?

4. **Architectural Decisions**: The task requires choosing between patterns or technologies
   - Example: "Add real-time updates" - WebSockets vs SSE vs polling
   - Example: "Implement state management" - Redux vs Context vs custom solution

5. **Multi-File Changes**: The task will likely touch more than 2-3 files
   - Example: "Refactor the authentication system"
   - Example: "Add a new API endpoint with tests"

6. **Unclear Requirements**: You need to explore before understanding the full scope
   - Example: "Make the app faster" - need to profile and identify bottlenecks
   - Example: "Fix the bug in checkout" - need to investigate root cause

7. **User Preferences Matter**: The implementation could reasonably go multiple ways
   - If you would use ${ASK_USER_QUESTION_TOOL_NAME} to clarify the approach, use EnterPlanMode instead
   - Plan mode lets you explore first, then present options with context

## When NOT to Use This Tool

Only skip EnterPlanMode for simple tasks:
- Single-line or few-line fixes (typos, obvious bugs, small tweaks)
- Adding a single function with clear requirements
- Tasks where the user has given very specific, detailed instructions
- Pure research/exploration tasks (use the Task tool with explore agent instead)

## What Happens in Plan Mode

In plan mode, you'll:
1. Thoroughly explore the codebase using Glob, Grep, and Read tools
2. Understand existing patterns and architecture
3. Design an implementation approach
4. Present your plan to the user for approval
5. Use ${ASK_USER_QUESTION_TOOL_NAME} if you need to clarify approaches
6. Exit plan mode with ExitPlanMode when ready to implement

## Examples

### GOOD - Use EnterPlanMode:
User: "Add user authentication to the app"
- Requires architectural decisions (session vs JWT, where to store tokens, middleware structure)

User: "Optimize the database queries"
- Multiple approaches possible, need to profile first, significant impact

User: "Implement dark mode"
- Architectural decision on theme system, affects many components

User: "Add a delete button to the user profile"
- Seems simple but involves: where to place it, confirmation dialog, API call, error handling, state updates

User: "Update the error handling in the API"
- Affects multiple files, user should approve the approach

### BAD - Don't use EnterPlanMode:
User: "Fix the typo in the README"
- Straightforward, no planning needed

User: "Add a console.log to debug this function"
- Simple, obvious implementation

User: "What files handle routing?"
- Research task, not implementation planning

## Important Notes

- This tool REQUIRES user approval - they must consent to entering plan mode
- If unsure whether to use it, err on the side of planning - it's better to get alignment upfront than to redo work
- Users appreciate being consulted before significant changes are made to their codebase
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| (无参数) | - | - | 仅触发模式切换 |

## 设计原理

### 1. 主动使用
**为什么**:
- 防止浪费时间在错误方向
- 确保与用户期望对齐
- 复杂任务需要规划

### 2. 详细的使用条件
**为什么**:
- 帮助 Agent 判断何时需要规划
- 避免过度规划简单任务
- 避免跳过需要规划的复杂任务

### 3. 需要用户批准
**为什么**:
- 用户控制工作流程
- 避免意外进入规划模式
- 尊重用户时间

### 4. 探索优先
**为什么**:
- 理解现有代码再做决策
- 发现潜在问题
- 提供有依据的方案

## 使用场景

### ✅ 应该使用
- 新功能实现
- 多种可行方案
- 架构决策
- 多文件修改
- 需求不明确

### ❌ 不应该使用
- 简单修复 (typo, 小 bug)
- 用户给出详细指令
- 纯研究任务 (使用 Task)

## 规划模式流程

```
1. EnterPlanMode
   ↓
2. 探索代码库 (Glob, Grep, Read)
   ↓
3. 设计实现方案
   ↓
4. 写入计划文件
   ↓
5. ExitPlanMode (请求批准)
   ↓
6. 用户批准 → 开始实现
```

## Sage 实现差异

Sage 的 EnterPlanModeTool 支持：
- 计划文件自动管理
- 计划版本历史
- 与任务系统集成
