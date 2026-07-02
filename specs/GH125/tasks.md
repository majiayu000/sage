# Task Plan

## Linked Issue

GH-125

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP125-T01` Owner: permissions. Done when: Ask 决策按稳定 audit_key 接入 `ApprovalCache` 与 `cache_ttl_ms`，TTL 内不重复 prompt，过期重新询问. Verify: `cargo test -p sage-core approval_cache permissions`。
- [ ] `SP125-T02` Owner: permissions. Done when: rules engine/handler 不再作为未接线生产语义存在，或被唯一 settings permission 入口消费. Verify: `rg -n "PermissionRuleEngine|RuleBasedHandler|PolicyHandler" crates/sage-core/src` 人工确认生产调用。
- [ ] `SP125-T03` Owner: tools. Done when: `ToolPermissionResult::Transform` 与 `PermissionDecision::Modify` 返回明确 unsupported error，不继续执行原工具调用. Verify: `cargo test -p sage-core parallel_executor permission`。
- [ ] `SP125-T04` Owner: permissions. Done when: decision_engine allow 优先级拆成显式 helper，混合 supplied/structured key 与多路径 preflight 测试覆盖. Verify: `cargo test -p sage-core permissions settings_permission`。

## 并行拆分

T01/T04 同在 permissions，串行更安全；T03 在 parallel executor，可并行；T02 依赖 T01/T04 最终决策后收口。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-core --all-targets -- -D warnings`
- `cargo test -p sage-core permissions`
- `cargo test -p sage-core parallel_executor`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH125`

## Handoff Notes

权限默认必须偏安全：不能为了减少弹窗而扩大 cache key；unsupported transform/modify 必须 fail closed。
