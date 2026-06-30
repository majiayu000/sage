# Product Spec

## Linked Issue

GH-87

## 用户问题

Sage 的 MCP 能力需要从直接配置和 extension package 两种来源统一管理。当前缺少可观察的 runtime 状态、认证状态、受控启动/重试和延迟工具发现，导致 MCP server 失败时很难知道是连接、认证、schema 还是配置来源的问题。

## 目标

- 合并 direct config 和 extension-sourced MCP servers，并保留 source metadata。
- 暴露 MCP server auth/OAuth 状态和授权提示。
- 支持 controlled startup、disconnect、retry 和 structured failure。
- 支持 deferred tool discovery，让工具列表可搜索但不要求启动时全量加载。
- 对连接、认证和 schema 失败返回结构化错误。

## 非目标

- 不硬编码 MCP server config。
- 不静默忽略已启用 server 的连接、认证或 schema 失败。
- 不实现 extension package 生命周期；这是 GH-86。
- 不放宽 GH-88 permission/sandbox 策略。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. 同一 MCP server 的来源、package id、config path 和启用状态必须可追踪。
2. 已启用 server 连接失败必须生成结构化状态，不得静默消失。
3. OAuth/认证未完成时，工具不可执行，并返回可恢复授权状态。
4. Remote stdio 或不受控 transport 必须 fail closed，除非明确实现了安全策略。
5. Deferred tools 必须带 server identity 和 discovery freshness metadata。
6. Direct config 与 package source 的 precedence 必须明确、可测试。

## 验收标准

- [ ] 支持 MCP source merge，区分 direct、package 和 future source。
- [ ] 支持 auth status、authorization prompt 和 recovery 状态。
- [ ] 支持 connect/disconnect/retry runtime action 或明确 fail-closed 响应。
- [ ] 支持 deferred tool list/search，不需要启动时连接所有 server。
- [ ] 覆盖 config-source merge、auth pending、connection failure、schema failure 和 disabled source 测试。

## 边界情况

- 两个 source 声明同一 server id：按 precedence 产生确定结果或冲突错误。
- Server 启用但认证缺失：server 状态为 auth_required，工具执行返回 structured auth error。
- Tool schema 解析失败：该 server 标记为 schema_error，其他 server 不受影响。
- Package 被禁用：其 MCP server 从 runtime source set 中移除。

## 发布说明

本 PR 仅添加 GH-87 focused spec。实现 PR 需要说明 MCP source precedence、auth recovery、transport 安全边界和 deferred discovery 行为。
