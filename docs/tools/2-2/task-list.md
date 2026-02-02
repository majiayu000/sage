# TaskList Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TaskList |
| Claude Code 版本 | 2.1.19 |
| 类别 | 任务管理 |
| Sage 实现 | `sage-tools/src/tools/task_mgmt/` |

## 功能描述

列出任务列表中的所有任务，查看整体进度和可用任务。

## 完整 Prompt

```markdown
Use this tool to list all tasks in the task list.

## When to Use This Tool

- To see what tasks are available to work on (status: 'pending', no owner, not blocked)
- To check overall progress on the project
- To find tasks that are blocked and need dependencies resolved
- After completing a task, to check for newly unblocked work or claim the next available task

## Output

Returns a summary of each task:
- **id**: Task identifier (use with TaskGet, TaskUpdate)
- **subject**: Brief description of the task
- **status**: 'pending', 'in_progress', or 'completed'
- **owner**: Agent ID if assigned, empty if available
- **blockedBy**: List of open task IDs that must be resolved first (tasks with blockedBy cannot be claimed until dependencies resolve)

Use TaskGet with a specific task ID to view full details including description and comments.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| (无参数) | - | - | 列出所有任务 |

## 输出字段

| 字段 | 说明 |
|------|------|
| id | 任务标识符 |
| subject | 任务简述 |
| status | 状态: pending/in_progress/completed |
| owner | 所有者 (空表示可认领) |
| blockedBy | 阻塞此任务的任务 ID 列表 |

## 设计原理

### 1. 摘要视图
**为什么**:
- 快速了解整体状态
- 不需要完整详情
- 减少输出量

### 2. 显示阻塞关系
**为什么**:
- 了解任务依赖
- 识别可执行任务
- 帮助解决阻塞

### 3. 完成后检查
**为什么**:
- 发现新解除阻塞的任务
- 认领下一个任务
- 保持工作流程

## 使用场景

### ✅ 应该使用
- 查看可用任务
- 检查项目进度
- 完成任务后找下一个
- 了解阻塞情况

### ❌ 不应该使用
- 需要任务完整详情 (使用 TaskGet)
- 更新任务 (使用 TaskUpdate)
- 创建任务 (使用 TaskCreate)

## 输出示例

```
Tasks:
#1. [completed] Set up project structure
#2. [in_progress] Implement user authentication (owner: agent-1)
#3. [pending] Add unit tests (blockedBy: #2)
#4. [pending] Deploy to staging
```

## Sage 实现差异

Sage 完整实现了 TaskList，与 Claude Code 兼容。
