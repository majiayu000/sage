# Task Plan

## Linked Issue

GH-130

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP130-T01` Owner: release. Done when: `scripts/release_gate.py` rejects tar symlink/hardlink/member escape or uses `filter="data"` with manual checks. Verify: release gate archive safety test.
- [ ] `SP130-T02` Owner: diagnostics. Done when: `PolicyAuditSummary.redacted_context` is redacted at construction or field is renamed so name/content match. Verify: `cargo test -p sage-core diagnostics`。
- [ ] `SP130-T03` Owner: permissions. Done when: multi-path filesystem permission inputs attach preflights to every path or reject multi-path explicitly, with regression test. Verify: `cargo test -p sage-core settings_permission`。
- [ ] `SP130-T04` Owner: permissions. Done when: path glob matcher handles case-insensitive filesystems safely or docs explicitly declare limitation. Verify: `cargo test -p sage-core permission cache`。

## 并行拆分

T01 Python script、T02 diagnostics、T03/T04 permissions 可分开做；T03/T04 同文件附近，建议同一实现 PR。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-core --all-targets -- -D warnings`
- `cargo test -p sage-core diagnostics settings_permission permission`
- release gate archive safety test command added by implementation PR
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH130`

## Handoff Notes

保持四项最小修复，不借机重构整个权限系统；权限架构收敛属于 GH125。
