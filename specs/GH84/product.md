# Product Spec

## Linked Issue

GH-84

## 用户问题

Sage 已有 subagent、Task 和 team 工具概念，但后台子代理、父子关系、等待/中断/列表和 follow-up 消息还没有统一到可恢复的 agent/thread graph。没有这个边界，parallel agent 工作无法可靠审计、恢复、关闭或继续对话。

## 目标

- 定义可持久化的 parent-child agent/thread graph。
- 让后台子代理启动、输出读取、wait/list/interrupt 和 follow-up messaging 使用同一图模型。
- 为 TaskOutput、agent path/mailbox 和子代理状态查询提供结构化 contract。
- 依赖 GH-82 ThreadStore lineage，而不是另建不可查询 registry。

## 非目标

- 不实现 OS 线程、tmux 或外部 orchestrator。
- 不做桌面 app 或 IDE UI。
- 不改变 GH-85 的 role/context fork 策略；本 issue 只定义 graph 和 messaging lifecycle。
- 不绕过 GH-88 permission profile。

## Behavior Invariants

1. 每个子代理必须记录 parent thread、child thread、spawn item 和 status。
2. `run_in_background=true` 必须返回可轮询 task/agent id；不能只模拟后台状态。
3. TaskOutput 必须读取结构化 status、event summary、stdout/stderr preview、error 和 final result。
4. wait/list/interrupt/follow-up 都必须使用同一 agent path/thread graph。
5. 失败、取消、超时、权限拒绝必须是结构化错误。
6. 子代理输出不能只存在内存 registry；restart 后至少能恢复 terminal state 和可用 summary。
7. 不允许静默丢失 child edge 或后台错误。

## 验收标准

- [ ] 子代理启动写入 parent-child edge，并可查询 direct children 与 descendants。
- [ ] `run_in_background=true` 会真实执行并返回可轮询 task id。
- [ ] `TaskOutput` 能读取子代理状态、stdout/event summary、错误和完成结果。
- [ ] 支持 wait/list/interrupt，失败、取消、超时都有结构化错误。
- [ ] 支持 agent path/mailbox 和触发新回合的 follow-up。

## 边界情况

- parent thread 被 archive：child 查询仍可按 ID 访问，但默认列表可隐藏 archived parent。
- child agent 启动后进程失败：graph edge 保留，status 变为 failed。
- follow-up 发送到 terminal child：返回结构化 invalid_state。
- restart 后后台任务仍运行/已退出：需要 reconciliation 规则。

## 发布说明

本 PR 仅添加 GH-84 focused spec。实现 PR 需要说明后台任务、持久化状态和子代理消息兼容性。
