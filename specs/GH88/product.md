# Product Spec

## Linked Issue

GH-88

## 用户问题

Sage 的 filesystem、network、exec、sandbox 和 approval 权限路径分散，Bash/后台进程等工具可能走不同判定链路。用户需要一个统一的 permission profile，保证所有工具和运行模式都经过同一套策略，并且危险失败不能降级成 warning 后继续执行。

## 目标

- 定义统一 `PermissionProfile`，覆盖 filesystem、network、exec、sandbox 和 approval。
- 建立 deny-before-allow 的中心决策引擎。
- 将 Bash 同步/后台执行接入同一 sandbox/permission 路径。
- 明确 macOS、Linux 等平台 sandbox 支持与 fail-closed 行为。
- 记录 approval cache、timeout、protected paths 和 network policy 行为。

## 非目标

- 不放宽默认 sandbox 或 approval 策略。
- 不用 warning-and-continue 处理危险权限失败。
- 不实现 extension package 运行时权限模型之外的 marketplace 能力。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. 所有工具执行前必须经过统一 permission decision。
2. Deny 规则优先级高于 allow 规则，不能被后续配置覆盖。
3. Bash、后台 Bash 和子进程不能绕过 sandbox/profile。
4. 受保护目录、workspace 外写入和 network deny 必须 fail closed。
5. Approval 超时或拒绝必须阻止对应动作。
6. 平台 sandbox 不可用时必须返回结构化 unsupported/fail-closed 状态。

## 验收标准

- [ ] 定义统一 permission profile DTO 和 merge/precedence 规则。
- [ ] 所有 filesystem/network/exec/sandbox decision 走中心引擎。
- [ ] Bash 同步和后台执行都使用平台 sandbox 或明确 fail-closed。
- [ ] Approval cache、deny、timeout 和 protected path 行为可测试。
- [ ] 覆盖 workspace allow、outside deny、protected dir deny、network off、approval allow/deny/timeout 测试。

## 边界情况

- 用户配置允许写入，但 system/profile deny 保护目录：最终 deny。
- Linux sandbox 未实现且请求 sandboxed exec：返回 fail-closed unsupported，不直接执行。
- Approval prompt 超时：动作被 deny，并记录可审计状态。
- Background Bash 重启或恢复：继续使用原 permission profile，不重新扩权。

## 发布说明

本 PR 仅添加 GH-88 focused spec。实现 PR 需要说明权限 precedence、平台支持矩阵、Bash 接入路径和 fail-closed 行为。
