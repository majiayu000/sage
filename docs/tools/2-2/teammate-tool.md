# TeammateTool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TeammateTool |
| Claude Code 版本 | 2.1.23 |
| 类别 | 团队协作 |
| Sage 实现 | 未实现 |

## 功能描述

管理团队和协调 Swarm 中的队友。用于团队操作、通信和任务分配。

## 完整 Prompt

```markdown
# TeammateTool

Manage teams and coordinate teammates in a swarm. Use this tool for team operations, communication, and task assignment. Note: To spawn new teammates, use the Task tool with `team_name` and `name` parameters.

## Operations

### spawnTeam - Create a Team

Create a new team to coordinate multiple agents working on a project. Teams have a 1:1 correspondence with task lists (Team = Project = TaskList).

```
{
  "operation": "spawnTeam",
  "team_name": "my-project",
  "description": "Working on feature X"
}
```

This creates:
- A team file at `~/.claude/teams/{team-name}.json`
- A corresponding task list directory at `~/.claude/tasks/{team-name}/`

### discoverTeams - Discover Available Teams

List teams that are available to join.

### requestJoin - Request to Join a Team

Send a join request to a team's leader.

### approveJoin - Approve a Join Request (Leader Only)

Accept a teammate's join request.

### rejectJoin - Reject a Join Request (Leader Only)

Decline a join request.

### cleanup - Clean Up Team Resources

Remove team and task directories when the swarm work is complete.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| operation | string | ✅ | 操作类型 |
| team_name | string | ❌ | 团队名称 |
| description | string | ❌ | 团队描述 |
| target_agent_id | string | ❌ | 目标 Agent ID |
| request_id | string | ❌ | 请求 ID |
| reason | string | ❌ | 拒绝原因 |

## 操作类型

| 操作 | 说明 | 权限 |
|------|------|------|
| spawnTeam | 创建团队 | 任何人 |
| discoverTeams | 发现可用团队 | 任何人 |
| requestJoin | 请求加入团队 | 任何人 |
| approveJoin | 批准加入请求 | 仅 Leader |
| rejectJoin | 拒绝加入请求 | 仅 Leader |
| cleanup | 清理团队资源 | 仅 Leader |

## 设计原理

### 1. Team = Project = TaskList
**为什么**:
- 简化概念模型
- 团队和任务自然关联
- 便于资源管理

### 2. 基于文件的团队配置
**为什么**:
- 持久化团队状态
- 便于调试和检查
- 支持跨进程共享

### 3. 加入请求机制
**为什么**:
- Leader 控制团队成员
- 避免未授权加入
- 支持能力描述

### 4. 自动消息传递
**为什么**:
- 队友消息自动送达
- 无需手动检查收件箱
- 简化通信流程

## 团队工作流程

```
1. spawnTeam - 创建团队和任务列表
   ↓
2. 使用 Task 工具创建队友 (带 team_name 和 name)
   ↓
3. 使用 TaskCreate 创建任务
   ↓
4. 使用 TaskUpdate 分配任务给队友
   ↓
5. 队友完成任务并标记完成
   ↓
6. cleanup - 清理团队资源
```

## 环境变量

队友进程会设置以下环境变量：

| 变量 | 说明 |
|------|------|
| CLAUDE_CODE_AGENT_ID | Agent 唯一标识 |
| CLAUDE_CODE_AGENT_TYPE | Agent 角色/类型 |
| CLAUDE_CODE_TEAM_NAME | 所属团队名称 |
| CLAUDE_CODE_PLAN_MODE_REQUIRED | 是否需要规划模式 |

## Sage 实现状态

Sage 目前未实现 TeammateTool。未来可能实现：
- 多 Agent 协作
- 任务分配系统
- Agent 间通信
