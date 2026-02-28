# ExitPlanMode Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | ExitPlanMode |
| Claude Code 版本 | 2.1.14 |
| 类别 | 规划工具 |
| Sage 实现 | `sage-tools/src/tools/planning/exit_plan_mode/` |

## 功能描述

退出规划模式，向用户展示计划并请求批准。

## 完整 Prompt

```markdown
Use this tool when you are in plan mode and have finished writing your plan to the plan file and are ready for user approval.

## How This Tool Works
- You should have already written your plan to the plan file specified in the plan mode system message
- This tool does NOT take the plan content as a parameter - it will read the plan from the file you wrote
- This tool simply signals that you're done planning and ready for the user to review and approve
- The user will see the contents of your plan file when they review it

## When to Use This Tool
IMPORTANT: Only use this tool when the task requires planning the implementation steps of a task that requires writing code. For research tasks where you're gathering information, searching files, reading files or in general trying to understand the codebase - do NOT use this tool.

## Before Using This Tool
Ensure your plan is complete and unambiguous:
- If you have unresolved questions about requirements or approach, use AskUserQuestion first (in earlier phases)
- Once your plan is finalized, use THIS tool to request approval

**Important:** Do NOT use AskUserQuestion to ask "Is this plan okay?" or "Should I proceed?" - that's exactly what THIS tool does. ExitPlanMode inherently requests user approval of your plan.

## Examples

1. Initial task: "Search for and understand the implementation of vim mode in the codebase" - Do not use the exit plan mode tool because you are not planning the implementation steps of a task.
2. Initial task: "Help me implement yank mode for vim" - Use the exit plan mode tool after you have finished planning the implementation steps of the task.
3. Initial task: "Add a new feature to handle user authentication" - If unsure about auth method (OAuth, JWT, etc.), use AskUserQuestion first, then use exit plan mode tool after clarifying the approach.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| (无参数) | - | - | 从计划文件读取内容 |

## 设计原理

### 1. 从文件读取计划
**为什么**:
- 计划可能很长，不适合作为参数
- 用户可以直接查看和编辑计划文件
- 保持工具接口简洁

### 2. 仅用于实现任务
**为什么**:
- 研究任务不需要批准
- 避免不必要的中断
- 区分探索和实现

### 3. 先解决问题再退出
**为什么**:
- 计划应该是完整的
- 避免来回修改
- 提高效率

### 4. 不要用 AskUserQuestion 确认计划
**为什么**:
- ExitPlanMode 本身就是请求批准
- 避免重复确认
- 保持流程清晰

## 使用场景

### ✅ 应该使用
- 完成实现计划后
- 准备开始编码前
- 需要用户批准方案时

### ❌ 不应该使用
- 研究/探索任务
- 计划还有未解决的问题
- 想问"计划可以吗？"

## 计划文件格式

```markdown
# Implementation Plan

## Summary
[Brief description of what will be implemented]

## Files to Modify
- path/to/file1.rs - [changes]
- path/to/file2.rs - [changes]

## Implementation Steps
1. [Step 1]
2. [Step 2]
3. [Step 3]

## Testing Strategy
- [How to verify the implementation]

## Risks and Considerations
- [Potential issues]
```

## Sage 实现差异

Sage 的 ExitPlanModeTool 支持：
- 计划格式验证
- 自动生成测试计划
- 与 CI/CD 集成
