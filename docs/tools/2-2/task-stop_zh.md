# TaskStop Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TaskStop |
| Claude Code 版本 | 2.1.x |
| 类别 | 后台任务 |
| Sage 实现 | `sage-tools/src/tools/process/` |

## 功能描述

停止正在运行的后台任务。

## 完整 Prompt

```markdown
- Stops a running background task by its ID
- Takes a task_id parameter identifying the task to stop
- Returns a success or failure status
- Use this tool when you need to terminate a long-running task
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| task_id | string | ✅ | 要停止的任务 ID |
| shell_id | string | ❌ | (已弃用) 使用 task_id |

## 设计原理

### 1. 通过 ID 停止
**为什么**:
- 精确定位任务
- 避免误停其他任务
- 与 TaskOutput 使用相同的 ID

### 2. 返回状态
**为什么**:
- 确认停止成功
- 处理停止失败的情况
- 提供反馈

### 3. 弃用 shell_id
**为什么**:
- 统一使用 task_id
- 向后兼容
- 简化 API

## 使用场景

### ✅ 应该使用
- 终止长时间运行的任务
- 取消不再需要的后台任务
- 停止卡住的进程

### ❌ 不应该使用
- 停止前台任务
- 任务已完成

## 使用示例

```json
{
  "task_id": "bg-shell-123"
}
```

## 输出

```json
{
  "success": true,
  "message": "Task bg-shell-123 stopped"
}
```

或失败时:

```json
{
  "success": false,
  "message": "Task not found or already completed"
}
```

## Sage 实现差异

Sage 通过 KillShellTool 提供类似功能，用于终止后台 shell 进程。
