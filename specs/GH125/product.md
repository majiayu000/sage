# Product Spec

## Linked Issue

GH-125

## 用户问题

权限子系统同时存在 `ApprovalCache`、rules engine、settings permission 与 parallel executor permission handler 多条语义。部分组件实现并导出但运行时不消费，`Transform`/`Modify` 又被当作 allow 继续执行，导致维护者以为权限能力已经接线，实际用户仍会重复弹窗或遭遇静默降级。

## 目标

- 保留一条生产权限判定入口，消除未接线的重复语义。
- 让 `permissions.approval.cache_ttl_ms` 对 Ask 决策生效，或删除该字段和相关死代码。
- `Transform`/`Modify` 不得静默等同 allow。
- 补齐 allow 匹配和多路径 preflight 行为测试。

## 非目标

- 不改变 settings.json 的用户可见权限规则语法。
- 不新增权限类型。
- 不重写完整权限架构。

## Behavior Invariants

1. 同一个 audit_key 在 `cache_ttl_ms` 内重复触发 Ask 时，不应重复弹窗；TTL 过期后重新询问。
2. workspace 内只保留一条被生产代码消费的权限判定路径。未被选择的 rules engine/handler 必须删除、降级为测试 fixture，或明确接入生产入口。
3. `ToolPermissionResult::Transform` 与 `PermissionDecision::Modify` 必须执行转换或返回 unsupported error，不能继续原调用。
4. decision_engine 的 allow 匹配必须用显式优先级表达，避免 supplied keys 与 structured keys 的 OR 组合误放行。
5. 多路径 filesystem input 的 preflight 附着策略必须显式化，且有测试证明每个路径不会绕过 deny。

## 验收标准

- [ ] `permissions.approval.cache_ttl_ms` 生效，或字段与 `ApprovalCache` 被删除。
- [ ] 生产代码中只有一条权限判定入口；未接线的 rules engine/handler 被接线或删除。
- [ ] Transform/Modify 不再静默当作 Allow。
- [ ] decision_engine 有混合 supplied/structured key 测试；多路径 preflight 行为有测试覆盖。

## 边界情况

- Ask handler 不存在且默认 deny：仍按现有安全默认拒绝，不写缓存。
- 缓存命中 deny：直接拒绝并记录 permission_denials。
- Transform/Modify 被外部 handler 返回：返回明确错误，提示该能力未实现或必须由后续任务实现。

## 发布说明

权限行为修复：重复 Ask 可在 TTL 内复用决定；unsupported transform/modify 不再被静默允许。

## 开放问题

- rules engine 是接入还是删除？建议本轮以 settings permission 为唯一生产入口，删除或收窄未接线 rules engine，避免第二套语法。
