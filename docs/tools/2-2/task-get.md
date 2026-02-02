# TaskGet Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TaskGet |
| Claude Code 版本 | 2.1.19 |
| 类别 | 任务管理 |
| Sage 实现 | `sage-tools/src/tools/task_mgmt/` |

## 功能描述

通过 ID 获取任务的完整详情，包括描述、依赖关系等。

## 完整 Prompt

```markdown
Use this tool to retrieve a task by its ID from the task list.

## When to Use This Tool

- When you need the full description and context before starting work on a task
- To understand task dependencies (what it blocks, what blocks it)
- After being assigned a task, to get complete requirements

## Output

Returns full task details:
- **subject**: Task title
- **description**: Detailed requirements and context
- **status**: 'pending', 'in_progress', or 'completed'
- **blocks**: Tasks waiting on this one to complete
- **blockedBy**: Tasks that must complete before this one can start

## Tips

- After fetching a task, verify its blockedBy list is empty before beginning work.
- Use TaskList to see all tasks in summary form.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| taskId | string | ✅ | 任务 ID |

## 输出字段

| 字段 | 说明 |
|------|------|
| subject | 任务标题 |
| description | 详细描述和上下文 |
| status | 状态: pending/in_progress/completed |
| blocks | 等待此任务完成的任务 |
| blockedBy | 必须先完成的任务 |
| owner | 任务所有者 |
| activeForm | 进行中显示文本 |
| metadata | 附加元数据 |

## 设计原理

### 1. 完整详情
**为什么**:
- TaskList 只显示摘要
- 开始工作前需要完整信息
- 包含验收标准等

### 2. 依赖关系
**为什么**:
- 了解任务的上下游
- 确认可以开始工作
- 规划执行顺序

### 3. 开始前检查
**为什么**:
- 确认 blockedBy 为空
- 避免开始被阻塞的任务
- 确保依赖已满足

## 使用场景

### ✅ 应该使用
- 开始任务前获取详情
- 理解任务依赖
- 被分配任务后了解需求

### ❌ 不应该使用
- 查看所有任务 (使用 TaskList)
- 更新任务 (使用 TaskUpdate)
- 创建任务 (使用 TaskCreate)

## 使用示例

```json
{"taskId": "3"}
```

输出:
```json
{
  "id": "3",
  "subject": "Add unit tests for authentication",
  "description": "Write unit tests for login, logout, and token refresh. Cover edge cases like expired tokens and invalid credentials.",
  "status": "pending",
  "activeForm": "Adding unit tests",
  "blocks": ["4"],
  "blockedBy": ["2"],
  "owner": null
}
```

## Sage 实现差异

Sage 完整实现了 TaskGet，与 Claude Code 兼容。
