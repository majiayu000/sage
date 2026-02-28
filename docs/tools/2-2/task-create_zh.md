# TaskCreate Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TaskCreate |
| Claude Code 版本 | 2.1.19 |
| 类别 | 任务管理 |
| Sage 实现 | `sage-tools/src/tools/task_mgmt/` |

## 功能描述

创建结构化的任务列表，支持任务依赖和状态跟踪。

## 完整 Prompt

```markdown
Use this tool to create a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.
It also helps the user understand the progress of the task and overall progress of their requests.

## When to Use This Tool

Use this tool proactively in these scenarios:

- Complex multi-step tasks - When a task requires 3 or more distinct steps or actions
- Non-trivial and complex tasks - Tasks that require careful planning or multiple operations
- Plan mode - When using plan mode, create a task list to track the work
- User explicitly requests todo list - When the user directly asks you to use the todo list
- User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)
- After receiving new instructions - Immediately capture user requirements as tasks
- When you start working on a task - Mark it as in_progress BEFORE beginning work
- After completing a task - Mark it as completed and add any new follow-up tasks discovered during implementation

## When NOT to Use This Tool

Skip using this tool when:
- There is only a single, straightforward task
- The task is trivial and tracking it provides no organizational benefit
- The task can be completed in less than 3 trivial steps
- The task is purely conversational or informational

NOTE that you should not use this tool if there is only one trivial task to do. In this case you are better off just doing the task directly.

## Task Fields

- **subject**: A brief, actionable title in imperative form (e.g., "Fix authentication bug in login flow")
- **description**: Detailed description of what needs to be done, including context and acceptance criteria
- **activeForm**: Present continuous form shown in spinner when task is in_progress (e.g., "Fixing authentication bug"). This is displayed to the user while you work on the task.

**IMPORTANT**: Always provide activeForm when creating tasks. The subject should be imperative ("Run tests") while activeForm should be present continuous ("Running tests"). All tasks are created with status `pending`.

## Tips

- Create tasks with clear, specific subjects that describe the outcome
- Include enough detail in the description for another agent to understand and complete the task
- After creating tasks, use TaskUpdate to set up dependencies (blocks/blockedBy) if needed
- Check TaskList first to avoid creating duplicate tasks
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| subject | string | ✅ | 任务标题 (祈使句) |
| description | string | ✅ | 详细描述 |
| activeForm | string | ✅ | 进行中显示文本 |
| metadata | object | ❌ | 附加元数据 |

## 设计原理

### 1. 分离 subject 和 description
**为什么**:
- subject: 简短标题，便于列表显示
- description: 详细信息，便于理解和执行
- 支持不同的显示场景

### 2. activeForm 必需
**为什么**:
- 用户看到的是进行中状态
- 现在进行时更自然 ("Running tests" vs "Run tests")
- 提供更好的反馈体验

### 3. 支持任务依赖
**为什么**:
- 复杂项目有任务顺序要求
- 使用 TaskUpdate 设置 blocks/blockedBy
- 自动管理任务执行顺序

### 4. 检查重复
**为什么**:
- 避免创建重复任务
- 先用 TaskList 查看现有任务
- 保持任务列表整洁

## 与 TodoWrite 的区别

| 特性 | TaskCreate | TodoWrite |
|------|------------|-----------|
| 任务依赖 | ✅ 支持 | ❌ 不支持 |
| 详细描述 | ✅ 分离字段 | ❌ 单一字段 |
| 元数据 | ✅ 支持 | ❌ 不支持 |
| 配套工具 | TaskUpdate, TaskList, TaskGet | 单一工具 |

## 相关工具

| 工具 | 用途 |
|------|------|
| TaskCreate | 创建新任务 |
| TaskUpdate | 更新任务状态和依赖 |
| TaskList | 列出所有任务 |
| TaskGet | 获取任务详情 |

## 使用示例

```json
{
  "subject": "Implement user authentication",
  "description": "Add JWT-based authentication with login/logout endpoints. Include token refresh mechanism and secure password hashing.",
  "activeForm": "Implementing user authentication"
}
```

## Sage 实现差异

Sage 完整实现了 TaskCreate 及相关工具，提供：
- 任务依赖图
- 任务优先级
- 任务分配 (owner)
- 任务历史记录
