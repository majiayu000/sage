# Task Plan

## Linked Issue

GH-128

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP128-T01` Owner: sdk. Done when: `ExecutionResult` 使用 SDK 自有窄化 `ExecutionConfigSummary`，不 public 暴露内部 `Config` 或 secret-bearing fields. Verify: `cargo test -p sage-sdk result`。
- [ ] `SP128-T02` Owner: sdk. Done when: `RunOptions` 成为唯一 public options 类型，覆盖 non_interactive，`UnifiedRunOptions` 删除且无 deprecated alias. Verify: `cargo test -p sage-sdk options` + search check。
- [ ] `SP128-T03` Owner: sdk. Done when: public execution 入口收敛为一族方法，examples/docs 更新到新 API. Verify: `cargo test -p sage-sdk --doc`。
- [ ] `SP128-T04` Owner: docs. Done when: SDK lib/version docs 删除 deprecated maintenance policy，并记录 breaking change. Verify: `rg -n "deprecated|UnifiedRunOptions|config_used" crates/sage-sdk` 人工确认只剩允许项。

## 并行拆分

T01/T02/T03 都在 SDK public API，串行更安全；T04 可最后做。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-sdk --all-targets -- -D warnings`
- `cargo test -p sage-sdk --all-targets`
- `cargo test -p sage-sdk --doc`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH128`

## Handoff Notes

不要添加 deprecated alias、compat shim 或 `#[allow(deprecated)]` 来保留旧 SDK API；这是明确的 breaking cleanup。
