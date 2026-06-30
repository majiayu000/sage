# Product Spec

## Linked Issue

GH-85

## 用户问题

Sage subagent 角色目前更接近固定 enum，缺少可配置 role、上下文继承策略和模型/工具范围覆盖。复杂任务需要明确控制子代理看到多少父会话上下文，以及它能使用哪些工具和模型配置。

## 目标

- 定义可配置 subagent role schema。
- 支持 `fork_context: none | all | last_n` 或等价策略。
- 允许 role 声明 prompt、tools、model、reasoning/profile 覆盖。
- 保持内置 `GeneralPurpose`、`Explore`、`Plan` 默认行为兼容。
- 明确 tool-scope intersection，防止子代理越权。

## 非目标

- 不做云端 agent marketplace。
- 不破坏默认内置角色。
- 不实现子代理图/后台消息；这是 GH-84。
- 不绕过 GH-88 permission profile。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. Role 配置必须有 schema 校验和路径边界。
2. 自定义 role 不能提升超出父 agent/profile 允许的工具范围。
3. `fork_context: none` 不携带父对话内容；`all` 携带完整可用上下文；`last_n` 只携带最近 N 轮。
4. 内置角色缺省行为保持兼容，不要求用户迁移配置。
5. 非法 role、路径逃逸、未知字段和工具越权必须 fail closed。
6. Role resolution 必须可审计，能说明 prompt/tools/model/profile 来源。

## 验收标准

- [ ] 支持 role 配置文件，包含 prompt、tools、model、reasoning/profile 覆盖。
- [ ] spawn 支持 `fork_context: none | all | last_n` 或等价策略。
- [ ] role 加载有路径边界和 schema 校验。
- [ ] 工具范围使用 parent/profile 交集，不能越权。
- [ ] 覆盖默认角色、custom role、非法 role、路径逃逸、工具越权测试。

## 边界情况

- role 文件缺失或损坏：返回结构化配置错误，不回退到错误角色。
- `last_n=0` 等价于 none 或明确 validation error；必须文档化。
- parent 没有某个工具权限：child role 即使声明该工具也不能使用。
- model/profile override 不可用：返回 structured unsupported 或 fallback 策略，不静默改用默认。

## 发布说明

本 PR 仅添加 GH-85 focused spec。实现 PR 需要说明 role 文件位置、schema 兼容性和工具权限安全边界。
