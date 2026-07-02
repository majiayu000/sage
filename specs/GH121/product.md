# Product Spec

## Linked Issue

GH-121

## 用户问题

destructive 确认弹窗允许用户编辑命令后批准，编辑后的命令会执行，但 PostToolUse hooks、会话审计记录和 undo 文件追踪看到的仍是编辑前的旧参数，可观测层与实际执行不一致。

## 目标

- hooks、会话记录观察到的 tool call 与实际执行的一致。
- 用户在确认中改写目标文件时，undo 追踪覆盖新目标。
- 内部确认标记（`user_confirmed`）不泄漏给观察方。

## 非目标

- 不改变 destructive 确认的交互与重询语义。
- 不改变 settings 权限决策逻辑。

## Behavior Invariants

1. destructive 确认中用户编辑命令并批准后，实际执行的必须是编辑后的命令。
2. post-execution hooks 与会话 tool 记录消费的 call 参数必须等于实际执行的参数。
3. 观察方看到的 call 不包含内部 `user_confirmed` 标记。
4. 用户编辑修改了文件目标时，新目标在执行前被纳入 undo 追踪。
5. 未发生编辑/确认的路径行为完全不变。

## 验收标准

- [ ] 回归测试证明编辑后的命令被执行且返回给调用方的 executed call 是编辑后的、无标记版本。
- [ ] 既有 destructive 确认与 settings recheck 测试全部通过。

## 边界情况

- 确认被拒绝/取消：返回错误结果，executed call 为当前请求（无标记）。
- 编辑后仍是 destructive：走既有循环重新确认，最终执行的 call 被传播。
- 编辑触发 settings deny：返回 blocked 结果。

## 发布说明

内部执行链路修复，无配置或 API 迁移需求（crate 内部方法签名变化）。
