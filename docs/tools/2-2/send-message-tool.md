# SendMessageTool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | SendMessageTool |
| Claude Code 版本 | 2.1.20 |
| 类别 | 团队协作 |
| Sage 实现 | 未实现 |

## 功能描述

在 Swarm 中向队友发送消息并处理协议请求/响应。

## 完整 Prompt

```markdown
# SendMessageTool

Send messages to teammates and handle protocol requests/responses in a swarm.

## Message Types

### type: "message" - Send a Direct Message

Send a message to a **single specific teammate**. You MUST specify the recipient.

**IMPORTANT for teammates**: Your plain text output is NOT visible to the team lead or other teammates. To communicate with anyone on your team, you **MUST** use this tool.

```
{
  "type": "message",
  "recipient": "researcher",
  "content": "Your message here"
}
```

### type: "broadcast" - Send Message to ALL Teammates (USE SPARINGLY)

Send the **same message to everyone** on the team at once.

**WARNING: Broadcasting is expensive.** Each broadcast sends a separate message to every teammate.

```
{
  "type": "broadcast",
  "content": "Message to send to all teammates"
}
```

### type: "request" - Send a Protocol Request

#### subtype: "shutdown" - Request a Teammate to Shut Down

```
{
  "type": "request",
  "subtype": "shutdown",
  "recipient": "researcher",
  "content": "Task complete, wrapping up the session"
}
```

### type: "response" - Respond to a Protocol Request

#### Approve/Reject Shutdown

```
{
  "type": "response",
  "subtype": "shutdown",
  "request_id": "abc-123",
  "approve": true
}
```

#### Approve/Reject Plan

```
{
  "type": "response",
  "subtype": "plan_approval",
  "request_id": "abc-123",
  "recipient": "researcher",
  "approve": true
}
```
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| type | string | ✅ | 消息类型: message/broadcast/request/response |
| recipient | string | ❌ | 接收者名称 (message/request 必需) |
| content | string | ❌ | 消息内容 |
| subtype | string | ❌ | 请求/响应子类型 |
| request_id | string | ❌ | 请求 ID (response 必需) |
| approve | boolean | ❌ | 是否批准 (response 必需) |

## 消息类型

| 类型 | 说明 | 使用场景 |
|------|------|----------|
| message | 直接消息 | 与特定队友通信 |
| broadcast | 广播消息 | 紧急团队通知 (谨慎使用) |
| request | 协议请求 | 关机请求等 |
| response | 协议响应 | 批准/拒绝请求 |

## 设计原理

### 1. 必须使用工具通信
**为什么**:
- 纯文本输出对队友不可见
- 确保消息正确传递
- 统一的通信机制

### 2. 广播谨慎使用
**为什么**:
- 每个广播发送 N 条消息
- 消耗 API 资源
- 成本随团队规模线性增长

### 3. 协议请求/响应
**为什么**:
- 结构化的控制流程
- 支持关机、计划批准等
- 可追踪的请求 ID

### 4. 使用名称而非 UUID
**为什么**:
- 更易读和记忆
- 便于调试
- 人性化的标识

## 使用场景

### ✅ 使用 message
- 回复特定队友
- 正常的来回通信
- 跟进某人的任务
- 分享只与部分人相关的发现

### ✅ 使用 broadcast
- 需要立即全团队注意的关键问题
- 影响所有人的重大公告

### ❌ 不要使用 broadcast
- 回复单个队友
- 常规通信
- 只与部分人相关的信息

## Sage 实现状态

Sage 目前未实现 SendMessageTool。未来可能实现：
- Agent 间消息传递
- 协议请求处理
- 广播机制
