# TaskUpdate Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TaskUpdate |
| Claude Code 版本 | 2.1.19 |
| 类别 | 任务管理 |
| Sage 实现 | `sage-tools/src/tools/task_mgmt/` |

## 功能描述

更新任务列表中的任务状态、详情和依赖关系。

## 完整 Prompt

```markdown
Use this tool to update a task in the task list.

## When to Use This Tool

**Mark tasks as resolved:**
- When you have completed the work described in a task
- When a task is no longer needed or has been superseded
- IMPORTANT: Always mark your assigned tasks as resolved when you finish them
- After resolving, call TaskList to find your next task

- ONLY mark a task as completed when you have FULLY accomplished it
- If you encounter errors, blockers, or cannot finish, keep the task as in_progress
- When blocked, create a new task describing what needs to be resolved
- Never mark a task as completed if:
  - Tests are failing
  - Implementation is partial
  - You encountered unresolved errors
  - You couldn't find necessary files or dependencies

**Update task details:**
- When requirements change or become clearer
- When establishing dependencies between tasks

## Fields You Can Update

- **status**: The task status (see Status Workflow below)
- **subject**: Change the task title (imperative form, e.g., "Run tests")
- **description**: Change the task description
- **activeForm**: Present continuous form shown in spinner when in_progress (e.g., "Running tests")
- **owner**: Change the task owner (agent name)
- **metadata**: Merge metadata keys into the task (set a key to null to delete it)
- **addBlocks**: Mark tasks that cannot start until this one completes
- **addBlockedBy**: Mark tasks that must complete before this one can start

## Status Workflow

Status progresses: `pending` → `in_progress` → `completed`

## Staleness

Make sure to read a task's latest state using `TaskGet` before updating it.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| taskId | string | ✅ | 任务 ID |
| status | string | ❌ | 新状态: pending/in_progress/completed |
| subject | string | ❌ | 新标题 |
| description | string | ❌ | 新描述 |
| activeForm | string | ❌ | 进行中显示文本 |
| owner | string | ❌ | 任务所有者 |
| metadata | object | ❌ | 元数据 (设为 null 删除) |
| addBlocks | array | ❌ | 被此任务阻塞的任务 ID |
| addBlockedBy | array | ❌ | 阻塞此任务的任务 ID |

## 设计原理

### 1. 严格的完成标准
**为什么**:
- 防止虚假完成
- 确保任务真正完成
- 遇到问题时保持 in_progress

### 2. 任务依赖
**为什么**:
- 支持复杂项目的任务顺序
- blocks: 此任务完成后才能开始的任务
- blockedBy: 必须先完成的任务

### 3. 先读取再更新
**为什么**:
- 避免覆盖其他更新
- 确保基于最新状态
- 防止竞态条件

### 4. 元数据支持
**为什么**:
- 存储自定义信息
- 灵活扩展
- 设为 null 可删除

## 状态流转

```
pending → in_progress → completed
   ↑          ↓
   └──────────┘ (遇到阻塞时回退)
```

## 使用示例

```json
// 开始任务
{"taskId": "1", "status": "in_progress"}

// 完成任务
{"taskId": "1", "status": "completed"}

// 认领任务
{"taskId": "1", "owner": "my-name"}

// 设置依赖
{"taskId": "2", "addBlockedBy": ["1"]}
```

## Sage 实现差异

Sage 完整实现了 TaskUpdate，与 Claude Code 兼容。
