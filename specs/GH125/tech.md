# Tech Spec

## Linked Issue

GH-125

## Product Spec

`specs/GH125/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Ask TTL cache | `crates/sage-core/src/permissions/approval_cache.rs` | `ApprovalCache` 有 TTL 单测并 re-export，但生产 Ask 循环未消费 | `cache_ttl_ms` 需要接线或删除 |
| settings permission | `crates/sage-core/src/agent/unified/settings_permission.rs`，`settings_permission_inputs.rs` | 当前 CLI/agent 可达的权限判定路径 | 建议作为唯一生产入口 |
| rules engine | `crates/sage-core/src/tools/permission/rules/*` | `PermissionRuleEngine`/handler 主要由测试调用 | 重复语义来源 |
| parallel executor | `crates/sage-core/src/tools/parallel_executor/executor/permission.rs` | `Transform`/`Modify` 分支返回 `None`，等价继续执行 | silent allow 风险 |
| decision engine | `crates/sage-core/src/permissions/decision_engine.rs` | allow 匹配存在 supplied/structured key 组合复杂逻辑 | 需要优先级与混合键回归测试 |

## 设计方案

1. **单一生产入口**：保留 `settings_permission`/`PermissionDecisionEngine` 作为生产权限入口。`tools/permission/rules` 若无生产需求则删除导出与未接线 handler；如维护者选择保留，必须由同一入口调用，且不得提供第二套并行判定。
2. **ApprovalCache 接线**：在 Ask 分支生成稳定 audit_key，读取 `ApprovalPermissionProfile.cache_ttl_ms`。Ask 前查 `ApprovalCache`；用户选择 allow/deny 后按 TTL 写入。cache key 必须包含 tool name、规范化 path/command、permission action，避免跨工具误命中。
3. **Transform/Modify 策略**：本 issue 不实现参数转换。遇到 `ToolPermissionResult::Transform` 或 `PermissionDecision::Modify` 时返回 `ToolResult::error` / permission error，并记录 error 级诊断。后续若实现转换，必须有转换前后参数审计。
4. **allow 匹配优先级**：把 supplied keys 与 structured keys 的匹配路径拆成命名 helper，明确优先级：deny preflight > exact structured allow > scoped allow > supplied compatibility key。混合键测试证明 supplied key 不能绕过 structured path deny。
5. **多路径 preflight**：`settings_permission_inputs.rs` 对每个 path 都附着必要 preflight，或明确只允许单路径工具进入该分支并加 debug/assert 守卫。优先选择每个路径独立附着 deny，避免未来多路径工具绕过。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | Ask branch + `ApprovalCache` | TTL 内一次 prompt，过期后重新 prompt 单测 |
| P2 | module exports/production calls | search check + compile tests，未接线 rules 不再公开或已由唯一入口消费 |
| P3 | parallel executor permission | Transform/Modify 返回 error 的单测 |
| P4 | decision_engine helpers | supplied/structured 混合键 deny/allow 回归测试 |
| P5 | settings_permission_inputs | 多路径 filesystem 每个 path 都执行 deny preflight 测试 |

## 数据流

tool call -> settings permission input builder -> deny preflight/scoped allow -> decision_engine -> Ask 分支查 `ApprovalCache` -> handler prompt -> cache result by TTL -> executor 执行或拒绝。unsupported Transform/Modify -> error result，不进入工具执行。

## 备选方案

- 完整接入 `PermissionRuleEngine`：可行，但必须把 settings 语法翻译进同一 engine。风险是本轮引入第二套权限语义，建议不选。
- 删除 `ApprovalCache` 和 TTL 字段：也满足声明-执行收敛，但会放弃 issue 期望的重复 Ask 缓存。

## 风险

- Security: cache key 过宽会误放行；必须包含 action/tool/path/command。
- Compatibility: Transform/Modify 从 silent allow 变 error，属于安全修复。
- Maintenance: 删除 rules exports 可能影响下游 crate，需要全 workspace 编译验证。

## 测试计划

- [ ] Unit tests: ApprovalCache production Ask TTL、Transform/Modify error、多路径 preflight、混合键 allow/deny。
- [ ] Search checks: rules engine 不再仅测试调用；`Transform support planned` / `Modify support planned` 注释消失。
- [ ] Integration tests: 重复 Ask 同 audit_key TTL 内不重复调用 handler。

## 回滚方案

关闭或移除 ApprovalCache 接线会恢复重复 Ask；unsupported Transform/Modify error 不应回滚为 silent allow，除非同时实现转换。
