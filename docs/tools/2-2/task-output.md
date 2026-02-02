# TaskOutput Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TaskOutput |
| Claude Code 版本 | 2.1.x |
| 类别 | 后台任务 |
| Sage 实现 | `sage-tools/src/tools/process/task_output/` |

## 功能描述

获取正在运行或已完成的后台任务的输出，包括后台 shell、Agent 和远程会话。

## 完整 Prompt

```markdown
- Retrieves output from a running or completed task (background shell, agent, or remote session)
- Takes a task_id parameter identifying the task
- Returns the task output along with status information
- Use block=true (default) to wait for task completion
- Use block=false for non-blocking check of current status
- Task IDs can be found using the /tasks command
- Works with all task types: background shells, async agents, and remote sessions
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| task_id | string | ✅ | 任务 ID |
| block | boolean | ✅ | 是否等待完成 |
| timeout | number | ✅ | 最大等待时间 (毫秒, 最大 600000) |

## 设计原理

### 1. 阻塞/非阻塞模式
**为什么**:
- block=true: 等待任务完成后返回
- block=false: 立即返回当前状态
- 灵活适应不同场景

### 2. 超时控制
**为什么**:
- 防止无限等待
- 最大 10 分钟
- 可配置的等待时间

### 3. 支持多种任务类型
**为什么**:
- 后台 shell 命令
- 异步 Agent
- 远程会话
- 统一的输出获取接口

## 使用场景

### ✅ 应该使用
- 检查后台任务进度
- 获取后台任务结果
- 等待长时间运行的任务

### ❌ 不应该使用
- 前台同步任务
- 任务管理 (使用 TaskList/TaskGet)

## 使用示例

```json
// 等待任务完成
{
  "task_id": "bg-shell-123",
  "block": true,
  "timeout": 60000
}

// 非阻塞检查状态
{
  "task_id": "agent-456",
  "block": false,
  "timeout": 1000
}
```

## 输出

```json
{
  "status": "completed",
  "output": "Build successful\n...",
  "exit_code": 0
}
```

## Sage 实现差异

Sage 实现了 TaskOutputTool，用于获取后台任务输出。
